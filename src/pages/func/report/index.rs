use crate::{
    bearer, commit_or_rollback,
    database::get_db,
    libs::{
        dser::{deser_empty_to_none, deserialize_time_scope},
        gen_id, TimeFormat, TIME,
    },
    log,
    pages::account::{get_user, User},
    parse_jwt_macro,
    perm::roles::role_to_name,
    Response, ResponseResult,
};
use axum::{
    extract::Path,
    http::HeaderMap,
    routing::{delete, post},
    Json, Router,
};
use mysql::{params, prelude::Queryable, PooledConn};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub fn index_router() -> Router {
    Router::new()
        .route("/report/add", post(add_report))
        .route("/report/read", post(read_report))
        .route("/report/update", post(update_report))
        .route("/report/delete/:id", delete(delete_report))
        .route("/report/infos", post(query_report))
}

#[derive(Deserialize, Debug)]
struct InsertReportParams {
    ty: u8,
    reviewer: String,
    cc: Vec<String>,
    #[serde(deserialize_with = "deser_empty_to_none")]
    ac: Option<String>,
    contents: String,
}

async fn add_report(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: InsertReportParams = serde_json::from_value(value)?;
    let user = get_user(&uid, &mut conn).await?;
    let role = role_to_name(&user.role);
    log!("{}-{} 发起添加报告请求", role, user.name);
    commit_or_rollback!(__insert_report, &mut conn, (&data, &user))?;
    log!("{}-{} 添加报告成功", role, user.name);
    Ok(Response::empty())
}

fn __insert_report(
    conn: &mut PooledConn,
    (params, user): (&InsertReportParams, &User),
) -> Result<(), Response> {
    let time = TIME::now()?;
    let id = gen_id(&time, "report");
    let send_time = mysql::Value::Bytes(time.format(TimeFormat::YYYYMMDD_HHMMSS).into_bytes());

    conn.exec_drop(
        "insert into report 
        (id, applicant, reviewer, ty, create_time, ac, contents,
            send_time, processing_time, opinion, status) 
        values 
        (:id, :applicant, :reviewer, :ty, :create_time, :ac, :contents,
            :send_time, null, '', 2)",
        params! {
                "id" => &id,
                "applicant" => &user.id,
                "reviewer" => &params.reviewer,
                "ty" => params.ty,
                "create_time" => time.format(TimeFormat::YYYYMMDD_HHMMSS),
                "ac" => &params.ac,
                "contents" => &params.contents,
                "send_time" => send_time
        },
    )?;
    for cc in &params.cc {
        conn.query_drop(format!(
            "INSERT IGNORE INTO report_cc (cc, report) VALUES ('{}', '{}')",
            cc, id
        ))?;
    }
    Ok(())
}

async fn delete_report(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    log!("{}-{}请求删除报告 {}", user.department, user.name, id);
    let key: Option<i32> = conn.query_first(format!(
        "select 1 from report where id = '{id}' and applicant='{uid}'"
    ))?;
    if key.is_none() {
        log!(
            "{}-{}删除报告 {} 失败，该报告不存在或申请人不是{}",
            user.department,
            user.name,
            id,
            user.name
        );
        return Err(Response::permission_denied());
    }
    commit_or_rollback!(__delete_report, &mut conn, &id)?;
    log!("{}-{}删除报告 {}成功", user.department, user.name, id);
    Ok(Response::empty())
}
fn __delete_report(conn: &mut PooledConn, id: &str) -> Result<(), Response> {
    conn.query_drop(format!("delete from report where id = '{id}' LIMIT 1"))?;
    conn.query_drop(format!("delete from report_cc where report = '{id}'"))?;
    Ok(())
}

#[derive(Deserialize)]
struct ReadParams {
    id: String,
    ok: bool,
    opinion: String,
}

async fn read_report(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    let data: ReadParams = serde_json::from_value(value)?;
    log!("{}-{} 请求批阅报告 {}", user.department, user.name, data.id);
    let report: Report = conn
        .query_first(format!(
            "select *, 1 as ac_name, 1 as applicant_name, 1 as reviewer_name 
        from report 
        where id ='{}'",
            data.id
        ))?
        .unwrap();
    if report.send_time.is_none() || report.processing_time.is_some() {
        return Err(Response::dissatisfy("未发送或已审批"));
    }
    if report.reviewer != uid {
        log!(
            "{}-{} 批阅报告 {} 失败，因为 {} 不是该报告的批阅人",
            user.department,
            user.name,
            data.id,
            user.name
        );
        return Err(Response::permission_denied());
    }
    let status = op::ternary!(data.ok => 0, 1);
    let process_time = TIME::now()?.format(TimeFormat::YYYYMMDD_HHMMSS);
    let update = format!(
        "update report set status={status}, 
        processing_time='{process_time}', opinion='{}' 
        WHERE id = '{}' AND reviewer='{uid}' 
        AND send_time IS NOT NULL 
        and processing_time is NULL LIMIT 1",
        data.opinion, data.id
    );
    // println!("{update}");
    conn.query_drop(update)?;
    log!("{}-{} 成功批阅报告 {}, 报告状态 {}", user.department, user.name, data.id, status);
    Ok(Response::empty())
}
#[derive(Deserialize)]
struct UpdateParams {
    id: String,
    ty: i32,
    reviewer: String,
    cc: Vec<String>,
    ac: String,
    contents: String,
}
async fn update_report(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    let data: UpdateParams = serde_json::from_value(value)?;
    log!("{} 发起修改报告请求, 报告id为{}", user, data.id);
    let key: Option<Option<String>> = conn.query_first(format!(
        "select processing_time from report where id = '{}' and applicant='{uid}'",
        data.id
    ))?;
    if let Some(r) = key {
        if r.is_some() {
            return Err(Response::dissatisfy("已批阅的报告无法修改"));
        }
    } else {
        return Err(Response::permission_denied());
    }
    __update_report(&mut conn, &data)?;
    log!(
        "{}-{} 修改报告 {} 成功",
        user.department,
        user.name,
        data.id
    );
    Ok(Response::empty())
}

fn __update_report(conn: &mut PooledConn, param: &UpdateParams) -> Result<(), Response> {
    conn.query_drop(format!(
        "update report set ty={}, reviewer='{}', ac='{}', contents='{}' 
        where id ='{}' limit 1",
        param.ty, param.reviewer, param.ac, param.contents, param.id
    ))?;
    conn.query_drop(format!("delete from report_cc where report='{}'", param.id))?;
    for cc in &param.cc {
        conn.query_drop(format!(
            "insert ignore into report_cc (cc, report) values ('{}', '{}')",
            cc, param.id
        ))?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct QueryParams {
    ty: u8,
    #[serde(deserialize_with = "deserialize_time_scope")]
    send_time: (String, String),
    status: i32,
    #[serde(deserialize_with = "deserialize_time_scope")]
    processing_time: (String, String),
    applicant: String,
    reviewer: String,
    cc: String,
    ac: String,
    limit: u32,
}

#[derive(mysql_common::prelude::FromRow, Serialize, Debug)]
struct Report {
    id: String,
    applicant: String,
    applicant_name: String,
    reviewer: String,
    reviewer_name: String,
    ac: Option<String>,
    ac_name: Option<String>,
    ty: i32,
    send_time: Option<String>,
    processing_time: Option<String>,
    opinion: String,
    contents: String,
    /// 0 审批通过，1 审批未通过，其他值表示未审批
    status: i32,
}

async fn query_report(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    log!("{}-{} 发起查询报告请求", user.department, user.name);
    let data: QueryParams = serde_json::from_value(value)?;
    let reports = __query(&mut conn, &data, &user)?;

    log!(
        "{}-{} 查询报告成功，共有{}条记录",
        user.department,
        user.name,
        reports.len()
    );
    Ok(Response::ok(json!(reports)))
}

fn __query_statement(
    send_time: &str,
    processing_time: &str,
    status: &str,
    param: &QueryParams,
) -> Result<String, Response> {
    let cc = if param.cc.is_empty() {
        String::new()
    } else {
        format!(
            "and exists (select 1 from report_cc rc where rc.report = r.id and rc.cc = '{}')",
            param.cc
        )
    };

    let applicant = if param.applicant.is_empty() {
        "is not null".to_owned()
    } else {
        format!("= '{}'", param.applicant)
    };
    let reviewer = if param.reviewer.is_empty() {
        "is not null".to_owned()
    } else {
        format!("= '{}'", param.reviewer)
    };

    let ty = match param.ty {
        0..=2 => format!("= {}", param.ty),
        3 => "is not null".to_owned(),
        _ => return Err(Response::invalid_value("ty值非法")),
    };
    let ac_filter = if param.ac.is_empty() {
        "left join customer c on c.id=r.ac".to_owned()
    } else {
        format!("join customer c on c.id=r.ac and c.id = '{}'", param.ac)
    };
    let query = format!(
        "select r.*, a.name as applicant_name, 
        rev.name as reviewer_name, 
        c.name as ac_name
        from report r
        join user a on r.applicant=a.id 
        join user rev on rev.id=r.reviewer
        {ac_filter}
        where (r.ty {ty}) 
            and ({send_time})
            and ({processing_time})
            and (r.status {status}) 
            and (r.reviewer {reviewer}) 
            and (r.applicant {applicant})
            {cc}
        order by r.send_time desc
        limit {}
        ",
        param.limit
    );
    // println!("{}", query);
    Ok(query)
}

fn __query(
    conn: &mut PooledConn,
    params: &QueryParams,
    user: &User,
) -> Result<Vec<Value>, Response> {
    if !params.reviewer.eq(&user.id) && !params.applicant.eq(&user.id) && !params.cc.eq(&user.id) {
        log!(
            "{}-{} 获取报告请求失败，原因权限不足",
            user.department,
            user.name
        );
        return Err(Response::permission_denied());
    }
    let st = &params.send_time;
    let pt = &params.processing_time;
    let pt = if pt.0.eq("0000-00-00") && pt.1.eq("9999-99-99") {
        "r.processing_time is null or r.processing_time is not null".to_string()
    } else {
        format!(
            "r.processing_time >= '{}' and r.processing_time <= '{}'",
            pt.0, pt.1
        )
    };
    let status = if params.status >= 3 {
        "is not null".to_string()
    } else {
        format!("= {}", params.status)
    };
    let st = format!("r.send_time >= '{}' and r.send_time <= '{}'", st.0, st.1);
    let query = __query_statement(&st, &pt, &status, params)?;
    let reports: Vec<Report> = conn.query(query)?;
    let mut data = Vec::new();
    for row in reports {
        let cc = conn.query_map(
            format!(
                "select rc.cc, u.name from report_cc rc
                    join user u on u.id=rc.cc
                    where rc.report='{}'",
                row.id
            ),
            |(cc, name): (String, String)| {
                json!({
                    "name": name,
                    "id": cc
                })
            },
        )?;
        data.push(json!({
            "id": row.id,
            "applicant": row.applicant,
            "applicant_name": row.applicant_name,
            "reviewer": row.reviewer,
            "reviewer_name": row.reviewer_name,
            "ac": match row.ac {
                Some(ac) if !ac.is_empty() => Some(ac),
                _ => None
            },
            "ac_name": row.ac_name,
            "ty": row.ty,
            "send_time": row.send_time,
            "processing_time": row.processing_time,
            "opinion": row.opinion,
            "status": row.status,
            "cc": cc,
            "contents": row.contents,
        }));
    }

    Ok(data)
}

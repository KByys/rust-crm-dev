-- 下拉框选项
CREATE TABLE IF NOT EXISTS drop_down_box (
    -- 下拉框名称，如 department
    name VARCHAR(30) NOT NULL,
    value VARCHAR(30) NOT NULL,
    create_time VARCHAR(25) NOT NULL,
    PRIMARY KEY (name, value)
);

INSERT
    IGNORE INTO drop_down_box (name, value, create_time)
VALUES
    ('payment', '现金', '0000-00-00 00:00:00'),
    ('payment', '银行转账', '0000-00-00 00:00:01'),
    ('payment', '对公转账', '0000-00-00 00:00:02');

INSERT
    IGNORE INTO drop_down_box (name, value, create_time)
VALUES
    ('invoice_type', '普通发票', '0000-00-00 00:00:00'),
    ('invoice_type', '专业发票', '0000-00-00 00:00:01'),
    ('invoice_type', '增值税专用发票', '0000-00-00 00:00:02');

INSERT
    IGNORE INTO drop_down_box (name, value, create_time)
VALUES
    ('department', '总经办', '0000-00-00 00:00:00');


CREATE TABLE IF NOT EXISTS custom_fields (
    -- 0 客户字段， 1 产品字段
    ty INT NOT NULL,
    -- 0 文本字段，1 时间字段，2下拉框字段
    display VARCHAR(2) NOT NULL,
    -- 字段显示文本
    value VARCHAR(30) NOT NULL,
    create_time VARCHAR(25) NOT NULL,
    PRIMARY KEY (ty, display, value)
);

CREATE TABLE IF NOT EXISTS custom_field_data (
    -- 0 客户字段， 1 产品字段
    fields INT NOT NULL,
    -- 0 文本字段，1 时间字段，2下拉框字段
    ty INT NOT NULL,
    -- 客户或产品对应的id
    id VARCHAR(150) NOT NULL,
    -- 字段显示文本
    display VARCHAR(30) NOT NULL,
    -- 对应的数据
    value VARCHAR(30) NOT NULL,
    -- create_time VARCHAR(25) NOT NULL,
    PRIMARY KEY (fields, ty, display, id)
);

-- 自定义字段下拉选项
CREATE TABLE IF NOT EXISTS custom_field_option (
    -- 0 客户字段， 1 产品字段
    ty INT NOT NULL,
    -- 显示的文本
    display VARCHAR(30) NOT NULL,
    -- 选项值
    value VARCHAR(30) NOT NULL,
    create_time VARCHAR(25) NOT NULL,
    PRIMARY KEY (ty, display, value)
);

-- 角色表
CREATE TABLE IF NOT EXISTS roles (
    id VARCHAR(50) NOT NULL,
    name VARCHAR(50) NOT NULL,
    PRIMARY KEY (id)
);

INSERT
    IGNORE INTO roles (id, name)
VALUES
    ('root', '总经理'),
    ('admin', '管理员'),
    ('manager', '主管'),
    ('salesman', '销售员');

-- 用户表
CREATE TABLE IF NOT EXISTS user(
    id VARCHAR(150) NOT NULL,
    smartphone VARCHAR(15) NOT NULL UNIQUE,
    name VARCHAR(20) NOT NULL,
    password BINARY(16) NOT NULL,
    department VARCHAR(30) NOT NULL,
    role VARCHAR(50) NOT NULL,
    sex INT NOT NULL,
    PRIMARY KEY (id)
);

-- 离职员工表
CREATE TABLE IF NOT EXISTS leaver (
    id VARCHAR(150) NOT NULL,
    PRIMARY KEY (id)
);

-- 客户表
CREATE TABLE IF NOT EXISTS customer (
    id VARCHAR(150) NOT NULL,
    smartphone VARCHAR(15) NOT NULL UNIQUE,
    name VARCHAR(50) NOT NULL,
    company VARCHAR(50) NOT NULL,
    is_share INT NOT NULL,
    sex INT NOT NULL,
    chat VARCHAR(50) NOT NULL,
    need TEXT NOT NULL,
    fax VARCHAR(50) NOT NULL,
    post VARCHAR(50) NOT NULL,
    industry VARCHAR(30) NOT NULL,
    birthday VARCHAR(10) NOT NULL,
    level VARCHAR(30) NOT NULL,
    create_time VARCHAR(25) NOT NULL,
    address VARCHAR(150) NOT NULL,
    -- 备注
    remark TEXT NOT NULL,
    -- 跟踪状态
    status VARCHAR(30),
    -- 来源
    source TEXT,
    -- 职务
    role VARCHAR(30),
    -- 客户类型
    ty VARCHAR(30),
    -- 客户标签
    tag VARCHAR(30),
    PRIMARY KEY (id)
);
-- 客户共享，先不管，不一定会写
create table if not exists customer_share (
    customer varchar(150) not null,
    share_salesman varchar(150) not null,
    primary key (customer, share_salesman)
);
-- 客户额外的信息
CREATE TABLE IF NOT EXISTS extra_customer_data (
    id VARCHAR(150) NOT NULL,
    salesman VARCHAR(150) NULL,
    -- 历史遗留，添加日期
    added_date VARCHAR(25) NULL,
    -- 上传拜访时间, 暂时的值，后面需要用联合查询替换
    -- last_visited_time VARCHAR(25) NULL,
    -- 已拜访次数
    -- visited_count INT NOT NULL,
    -- 上次成交时间 暂时的值，后面需要用联合查询替换，历史遗留
    last_transaction_time VARCHAR(25) NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (id) REFERENCES customer(id),
    FOREIGN KEY (salesman) REFERENCES user(id)
);

-- 客户同事表
CREATE TABLE IF NOT EXISTS customer_colleague(
    id VARCHAR(150) NOT NULL,
    customer VARCHAR(150) NOT NULL,
    phone VARCHAR(15) NOT NULL,
    name VARCHAR(10) NOT NULL,
    create_time VARCHAR(25),
    PRIMARY KEY(id)
);

-- 客户预约表，暂时不管
CREATE TABLE IF NOT EXISTS appointment(
    id VARCHAR(150) NOT NULL,
    applicant VARCHAR(150) NOT NULL,
    salesman VARCHAR(150) NULL,
    customer VARCHAR(150) NULL,
    appointment VARCHAR(25) NOT NULL,
    finish_time VARCHAR(25),
    theme VARCHAR(30),
    content TEXT,
    PRIMARY KEY (id)
);
-- 预约评论，不用管
CREATE TABLE IF NOT EXISTS appoint_comment (
    id VARCHAR(150) NOT NULL,
    applicant VARCHAR(150) NOT NULL,
    appoint VARCHAR(150) NOT NULL,
    create_time VARCHAR(25) NOT NULL,
    comment TEXT,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS token(
    -- 历史遗留，token类型，现在没有用处
    ty INT NOT NULL,

    -- 用户id
    id VARCHAR(150) NOT NULL,
    -- 时间戳，如果id对应的token的签发时间小于该时间戳
    -- 则该token无效
    tbn BIGINT NULL,
    PRIMARY KEY(ty, id)
);

-- 产品表， 之后大修改！！！！！！！！！！！！！！！！！！！
-- num 编号
-- cover 封面的地址
CREATE TABLE IF NOT EXISTS product(
    id VARCHAR(150) NOT NULL,
    num VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(50) NOT NULL,
    specification VARCHAR(10) NOT NULL,
    cover VARCHAR(150) NULL,
    model VARCHAR(20) NOT NULL,
    unit VARCHAR(30) NOT NULL,
    product_type VARCHAR(30) NOT NULL,
    price FLOAT NOT NULL,
    create_time VARCHAR(25) NOT NULL,
    barcode VARCHAR(50) NOT NULL,
    explanation TEXT,
    purchase_price FLOAT NOT NULL,
    PRIMARY KEY (id)
);
-- 产品库存，之后会调整
CREATE TABLE IF NOT EXISTS product_store(
    product VARCHAR(150) NOT NULL,
    storehouse VARCHAR(30) NOT NULL,
    amount INT NOT NULL,
    PRIMARY KEY (product, storehouse)
);

-- 产品编号，用于记录顺序
CREATE TABLE IF NOT EXISTS product_num(
    name VARCHAR(100) NOT NULL,
    num INT NOT NULL,
    PRIMARY KEY (name)
);

-- 报告表
CREATE TABLE IF NOT EXISTS report(
    id VARCHAR (150) NOT NULL,
    applicant VARCHAR(150) NOT NULL,
    reviewer VARCHAR(150) NOT NULL,
    -- 0 日报，1 周报，2 月报
    ty INT NOT NULL,
    create_time VARCHAR (25) NOT NULL,
    -- 关联客户
    ac VARCHAR(150) NULL,
    contents TEXT NOT NULL,
    send_time VARCHAR (25) NULL,
    processing_time VARCHAR(25) NULL,
    opinion TEXT NULL,
    -- 0 审批通过，1 不通过, 2未审批，
    status INT NOT NULL,
    PRIMARY KEY (id)
);

-- 报告抄送人
CREATE TABLE IF NOT EXISTS report_cc (
    cc VARCHAR(150) NOT NULL,
    report VARCHAR(150) NOT NULL,
    PRIMARY KEY (cc, report)
);


-- 记录订单和发票的编号顺序
CREATE TABLE IF NOT EXISTS order_num(
    name VARCHAR(150) NOT NULL,
    -- 0 订单， 1 发票
    ty INT NOT NULL,
    num INTEGER NOT NULL,
    PRIMARY KEY (name, ty)
);

CREATE TABLE IF NOT EXISTS order_data(
    id VARCHAR(150) NOT NULL,
    number VARCHAR(150) NOT NULL UNIQUE,
    create_time VARCHAR(25) NOT NULL,
    status INT NOT NULL,
    ty VARCHAR(30) NOT NULL,
    file VARCHAR(150) NULL,
    receipt_account VARCHAR(50),
    salesman VARCHAR(150) NOT NULL,
    payment_method VARCHAR(30),
    customer VARCHAR(150) NOT NULL,
    address TEXT,
    purchase_unit VARCHAR(100),
    transaction_date VARCHAR(25) NULL,
    invoice_required INT NOT NULL,
    comment TEXT NOT NULL,
    shipped INT NOT NULL,
    shipped_date VARCHAR(25) NULL,
    shipped_storehouse VARCHAR(30) NULL,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS order_product(
    order_id VARCHAR(150) NOT NULL,
    id VARCHAR(150) NOT NULL,
    price FLOAT NOT NULL,
    discount FLOAT NOT NULL,
    amount INT NOT NULL,
    PRIMARY KEY (order_id, id)
);

CREATE TABLE IF NOT EXISTS invoice(
    order_id VARCHAR(150) NOT NULL,
    number VARCHAR(150) NOT NULL,
    title VARCHAR(30) NOT NULL,
    deadline VARCHAR(30),
    description TEXT,
    PRIMARY KEY (number)
);

CREATE TABLE IF NOT EXISTS order_instalment(
    order_id VARCHAR(150) NOT NULL,
    interest FLOAT NOT NULL,
    original_amount FLOAT NOT NULL,
    inv_index INT NOT NULL,
    date VARCHAR(25) NOT NULL,
    finish INT NOT NULL,
    PRIMARY KEY (order_id, inv_index)
);
create table if not exists storehouse(
    id VARCHAR(150) NOT NULL,
    name VARCHAR(100) NOT NULL UNIQUE,
    create_time VARCHAR(25) NOT NULL,
    description text NOT NULL,
    PRIMARY KEY (id)
);

INSERT
    IGNORE INTO storehouse (id, name, create_time, description)
VALUES
    ('main_storehouse', '主仓库', '0000-00-00 00:00:00', '默认生成');

create table if not exists supper(
    id VARCHAR(150) NOT NULL,
    company VARCHAR(100) NOT NULL UNIQUE,
    contact VARCHAR(100) NOT NULL,
    create_time VARCHAR(25) NOT NULL,
    phone VARCHAR(20) NOT NULL,
    mobile_phone VARCHAR(20) NOT NULL,
    address VARCHAR(100) NOT NULL,
    blank VARCHAR(200) NOT NULL,
    account VARCHAR(100) NOT NULL,
    remark text NOT NULL,
    PRIMARY KEY (id)
);


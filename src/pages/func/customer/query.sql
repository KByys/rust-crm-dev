
CREATE TABLE IF NOT EXISTS user (
    id VARCHAR(150) NOT NULL,
    smartphone VARCHAR(15) NOT NULL UNIQUE,
    name VARCHAR(20) NOT NULL,
    password BINARY(16) NOT NULL,
    department VARCHAR(30) NOT NULL,
    role VARCHAR(50) NOT NULL,
    sex INT NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (role) REFERENCES roles(id)
);


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

CREATE TABLE IF NOT EXISTS extra_customer_data (
    id VARCHAR(150) NOT NULL,
    salesman VARCHAR(150) NULL,
    -- 下次拜访时间
    next_visit_time VARCHAR(50) NULL,
    -- 上传拜访时间
    last_visited_time VARCHAR(50) NULL,
    -- 已拜访次数
    visited_count INT NOT NULL,
    -- 上次成交时间
    last_transaction_time VARCHAR(50) NULL,
    push_to_sea_date VARCHAR(50) NULL,
    pop_from_sea_date VARCHAR(50) NULL
    PRIMARY KEY (id),
    FOREIGN KEY (id) REFERENCES customer(id),
    FOREIGN KEY (salesman) REFERENCES user(id)
);


SELECT c.* FROM customer c WHERE (c.status {})
    AND (c.salesman {}) AND (c.is_share {}) AND (c.ty {}) AND (c.next_visit_time {})
    AND (c.last_visited_time {}) AND (c.create_time {})

    AND
    (c.push_to_sea_date IS NULL OR c.pop_from_sea_date > c.push_to_sea_date)
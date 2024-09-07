//! 新的权限设置，还没有完全设计好
//! 
//! 
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[macro_export]
macro_rules! pstr {
    (@1 $t:expr) => {
        $t.to_string()
    };
    (@op $t:expr) => {
        $t.map(|v|
            $crate::pstr!(@1 v)
        )
    };
    (@arr $t:expr) => {
        $t.iter().map(|v| serde_json::json!(v)).collect()
    };
}

#[macro_export]
macro_rules! gen_perm {
    (@map ($ ($arg:expr, ) *)) => {
        {
            [$( ($arg.value.clone(), $arg)), +].into_iter().collect::<dashmap::DashMap<_, _>>()
        }
    };
    // 入口
    ( $(@root {
        ($root_name:expr, $root_value:expr, $root_selected:expr, $root_comment:expr) =>
        {
            $(
                #1 ( $name1:expr, $value1:expr, $data1:expr, $selected1:expr, $comment1:expr)
            ),*
            $(
                #2 ($name2:expr, $value2:expr, $data2:expr, $selected2:expr, $comment2:expr) => {
                    $( #3 ( $name21:expr, $value21:expr, $data21:expr, $selected21:expr, $comment21:expr)), +
                }
            ), *
        }

    }), +) => {
        {
            let empty: [i32; 0] = [];
            $crate::gen_perm![
                @map
                ($(
                    (

                        $crate::gen_perm!(
                            @perm
                            $root_name, &empty, $root_value, $root_selected, 0, "",
                            $crate::gen_perm![
                                @map
                                (
                                    $(
                                        $crate::gen_perm!(
                                            @perm
                                            $name1, $data1, $value1, $selected1, 1, $root_value,
                                            dashmap::DashMap::new(), $comment1
                                        ),
                                    ) *
                                    $(
                                        $crate::gen_perm!(
                                            @perm
                                            $name2, $data2, $value2, $selected2, 1, $root_value,
                                            $crate::gen_perm![
                                                @map
                                                ($(
                                                    $crate::gen_perm!(
                                                        @perm
                                                        $name21, $data21, $value21, $selected21, 1, $value2,
                                                        dashmap::DashMap::new(), $comment21
                                                    ),
                                                ) +)
                                            ],
                                            $comment2
                                        ),
                                    )*
                                )
                                
                            ],
                            $root_comment
                        )
                    ),
                ) *)
            ]
        }
    };
    (@perm $name:expr, $data:expr, $value:expr, $selected:expr, $level:expr, $parent:expr, $children:expr, $comment:expr) => {
        $crate::libs::perm::Permission {
            name: $crate::pstr!(@1 $name),
            value: $crate::pstr!(@1 $value),
            selected: $selected,
            data: $crate::pstr!(@arr $data),
            level: $level,
            children: $children,
            parent: if $parent.is_empty() { None } else { Some( $crate::pstr!(@1 $parent) )},
            comment: $crate::pstr!(@1 $comment)
        }
    }
}

pub fn default_role_perms() -> DashMap<String, Permission> {
    let _empty: [i32; 0] = [];
    gen_perm![
        @root {
            ("职务管理权限组", "role", 0, "管理职务的权限组") => {
                #1 ("创建职务", "cr1", &_empty, 0, "需要指定可创建的职务"),
                #1 ("删除职务", "dr1", &_empty, 0, "需要指定可删除的职务"),
                #1 ("更改职务", "ur1", &_empty, 0, "需要指定可更改的职务"),
                #1 ("职务调动", "cr2", &_empty, 0, "需要指定可调动的职务，功能相当于员工的升职或降级")
            }
        },
        @root {
            ("账号权限组", "account", 0, "管理用户账号的权限组") => {
                #1 ("创建账号", "ca1", &_empty, 0, "需要指定可创建哪一类(职务)的员工账号和允许的部门范围"),
                #1 ("删除账号", "da1", &_empty, 0, "需要指定可删除哪一类(职务)的员工账号和允许的部门范围")
            }
        },
        @root {
            ("客户管理权限组", "customer", 0, "管理客户的权限组，不勾选无法使用客户模块") => {
                #1 ("录入客户数据", "cc1", &_empty, 0, "不勾选无法添加客户"),
                #1 ("删除客户数据", "dc1", &_empty, 0, "勾选后仅可删除自己的客户，不勾选无法删除客户"),
                #1 ("修改客户数据", "uc1", &_empty, 0, "勾选后仅可修改自己的客户数据，不勾选无法修改客户数据"),
                #1 ("查询客户数据", "qc1", &_empty, 0, "不勾选仅可查看自己和共享的客户数据，勾选后默认可查看本部门的客户数据，也可设置为可查看全公司的客户数据"),
                #1 ("导出客户数据", "exc1", &_empty, 0, "勾选后可将客户数据导出成表格"),
                #1 ("安排客户拜访", "aa1", &_empty, 0, "勾选后可给其他业务员安排客户拜访")
            }
        },
        @root {
            ("公海权限组", "sea", 0, "管理公海的权限组，不勾选无法使用公海模块") => {
                #1 ("释放客户", "scc1", &_empty, 0, "勾选后可释放客户到公司公海，不勾选仅可释放客户到部门公海"),
                #1 ("领取公海客户", "sdc1", &_empty, 0, "勾选后可从公海中领取客户（需要勾选客户模块的权限组）")
            }
        },
        @root {
            ("库房权限组", "storehouse", 0, "管理库房的权限组，不勾选无法使用库房模块") => {
                #2 ("产品管理", "sp1", &_empty, 0, "") => {
                    #3 ("录入产品", "sp1_1", &_empty, 0, "勾选后可录入产品信息"),
                    #3 ("调整产品信息", "sp1_2", &_empty, 0, "勾选后可调整产品信息（不包括库存）"),
                    #3 ("调整产品库存", "sp1_3", &_empty, 0, "勾选后可调整产品库存"),
                    #3 ("删除产品", "sp1_4", &_empty, 0, "勾选后可删除产品")
                },
                #2 ("仓库管理", "sh1", &_empty, 0, "") => {
                    #3 ("添加仓库", "sh1_1", &_empty, 0, "勾选后可添加产品仓库"),
                    #3 ("更新仓库信息", "sh1_2", &_empty, 0, "勾选后可"),
                    #3 ("删除仓库", "sh1_3", &_empty, 0, "勾选后可调整产品库存")
                }
            }
        },
        @root {
            ("其他权限组", "other", 0, "零散的权限设置") => {
                #2 ("产品管理", "sp1", &_empty, 0, "") => {
                    #3 ("录入产品", "sp1_1", &_empty, 0, "勾选后可录入产品信息"),
                    #3 ("调整产品信息", "sp1_2", &_empty, 0, "勾选后可调整产品信息（不包括库存）"),
                    #3 ("调整产品库存", "sp1_3", &_empty, 0, "勾选后可调整产品库存"),
                    #3 ("删除产品", "sp1_4", &_empty, 0, "勾选后可删除产品")
                },
                #2 ("仓库管理", "sh1", &_empty, 0, "") => {
                    #3 ("添加仓库", "sh1_1", &_empty, 0, "勾选后可添加产品仓库"),
                    #3 ("更新仓库信息", "sh1_2", &_empty, 0, "勾选后可"),
                    #3 ("删除仓库", "sh1_3", &_empty, 0, "勾选后可调整产品库存")
                }
            }
        }
    ]
}

#[derive(Deserialize, Serialize)]
pub struct Permission {
    /// 显示的名称
    pub name: String,
    pub value: String,
    pub data: Vec<Value>,
    pub selected: i32,
    pub level: i32,
    pub parent: Option<String>,
    pub children: DashMap<String, Permission>,
    pub comment: String,
}

impl Permission {
    pub fn is_selected(&self) -> bool {
        self.selected == 1
    }
}

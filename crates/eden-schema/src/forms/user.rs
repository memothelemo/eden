use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct UpdateUserForm {
    pub developer_mode: Option<bool>,
}

pub fn remove_if_exists<'a, T: PartialEq + Eq>(vec: &'a mut Vec<T>, value: &'a T) {
    let id = vec
        .iter()
        .enumerate()
        .find(|element| element.1.eq(value))
        .map(|v| v.0);

    if let Some(id) = id {
        vec.remove(id);
    }
}

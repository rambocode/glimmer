//! `IBusText`（`src/ibustext.c`）：`(s a{sv} s v)`，最后的 `v` 里是 `IBusAttrList`（`src/ibusattrlist.c`：`(s a{sv} av)`）。

use zbus::zvariant::Value;

use super::attribute::Attribute;
use super::{boxed, serializable};

/// 带样式的一段文字。
pub fn text(content: &str, attributes: &[Attribute]) -> Value<'static> {
    let attributes: Vec<Value<'static>> = attributes
        .iter()
        .map(|attribute| boxed(attribute.to_value()))
        .collect();
    let list = serializable("IBusAttrList", vec![Value::from(attributes)]);
    serializable(
        "IBusText",
        vec![Value::from(content.to_owned()), boxed(list)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_signature_matches_ibus() {
        let value = text("你好", &[Attribute::underline(0, 2)]);
        assert_eq!(value.value_signature().to_string(), "(sa{sv}sv)");
        let Value::Structure(structure) = &value else {
            panic!("IBusText 应是结构体");
        };
        let fields = structure.fields();
        assert_eq!(fields[0], Value::from("IBusText"));
        assert_eq!(fields[2], Value::from("你好"));
        let Value::Value(list) = &fields[3] else {
            panic!("属性表应包在 variant 里");
        };
        assert_eq!(list.value_signature().to_string(), "(sa{sv}av)");
    }

    #[test]
    fn attribute_signature_matches_ibus() {
        let value = Attribute::foreground(0x808080, 1, 3).to_value();
        assert_eq!(value.value_signature().to_string(), "(sa{sv}uuuu)");
    }
}

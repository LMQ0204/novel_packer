use visdom::Vis;
use visdom::types::BoxDynError;

// 从 HTML 字符串创建 Elements 对象
pub fn load_html(html: &str) -> Result<visdom::types::Elements, BoxDynError> {
    Vis::load(html)
}

// 将 Elements 对象转换为 HTML 字符串
pub fn to_html(elements: &visdom::types::Elements) -> String {
    elements.html()
}

// 获取元素的文本内容
pub fn get_text(elements: &visdom::types::Elements, selector: &str) -> String {
    let found = elements.find(selector);
    found.text().to_string()
}

// 获取元素的 HTML 内容
pub fn get_html(elements: &visdom::types::Elements, selector: &str) -> String {
    let found = elements.find(selector);
    found.html().to_string()
}

// 检查元素是否存在
pub fn exists(elements: &visdom::types::Elements, selector: &str) -> bool {
    let found = elements.find(selector);
    !found.is_empty()
}

// 获取元素数量
pub fn count(elements: &visdom::types::Elements, selector: &str) -> usize {
    let found = elements.find(selector);
    found.length()
}

// 修改元素的属性
pub fn set_attribute(elements: &visdom::types::Elements, selector: &str, attr: &str, value: &str) {
    let mut found = elements.find(selector);
    found.set_attr(attr, Some(value));
}

// 移除元素的属性
pub fn remove_attribute(elements: &visdom::types::Elements, selector: &str, attr: &str) {
    let mut found = elements.find(selector);
    found.remove_attr(attr);
}

// 设置元素的文本内容
pub fn set_text(elements: &visdom::types::Elements, selector: &str, text: &str) {
    let mut found = elements.find(selector);
    found.set_text(text);
}

// 设置元素的 HTML 内容
pub fn set_html(elements: &visdom::types::Elements, selector: &str, html: &str) {
    let mut found = elements.find(selector);
    found.set_html(html);
}

// 添加 CSS 类
pub fn add_class(elements: &visdom::types::Elements, selector: &str, class_name: &str) {
    let mut found = elements.find(selector);
    found.add_class(class_name);
}

// 移除 CSS 类
pub fn remove_class(elements: &visdom::types::Elements, selector: &str, class_name: &str) {
    let mut found = elements.find(selector);
    found.remove_class(class_name);
}

// 切换 CSS 类
pub fn toggle_class(elements: &visdom::types::Elements, selector: &str, class_name: &str) {
    let mut found = elements.find(selector);
    found.toggle_class(class_name);
}

// 删除匹配选择器的元素（需要启用 "destroy" 特性）
pub fn remove_elements(elements: &visdom::types::Elements, selector: &str) {
    let mut found = elements.find(selector);
    found.remove();
}

// 清空匹配选择器的元素内容（需要启用 "destroy" 特性）
pub fn empty_elements(elements: &visdom::types::Elements, selector: &str) {
    let mut found = elements.find(selector);
    found.empty();
}

// 在匹配的元素内部末尾添加新元素（需要启用 "insertion" 特性）
pub fn append_html(elements: &visdom::types::Elements, selector: &str, html: &str) -> Result<(), BoxDynError> {
    let mut parent_elements = elements.find(selector);
    let mut new_elements = Vis::load(html)?;
    parent_elements.append(&mut new_elements);
    Ok(())
}

// 在匹配的元素内部开头添加新元素（需要启用 "insertion" 特性）
pub fn prepend_html(elements: &visdom::types::Elements, selector: &str, html: &str) -> Result<(), BoxDynError> {
    let mut parent_elements = elements.find(selector);
    let mut new_elements = Vis::load(html)?;
    parent_elements.prepend(&mut new_elements);
    Ok(())
}

// 在匹配的元素之后添加新元素（需要启用 "insertion" 特性）
pub fn after_html(elements: &visdom::types::Elements, selector: &str, html: &str) -> Result<(), BoxDynError> {
    let mut target_elements = elements.find(selector);
    let mut new_elements = Vis::load(html)?;
    target_elements.after(&mut new_elements);
    Ok(())
}

// 在匹配的元素之前添加新元素（需要启用 "insertion" 特性）
pub fn before_html(elements: &visdom::types::Elements, selector: &str, html: &str) -> Result<(), BoxDynError> {
    let mut target_elements = elements.find(selector);
    let mut new_elements = Vis::load(html)?;
    target_elements.before(&mut new_elements);
    Ok(())
}

// 替换匹配的元素（需要启用 "insertion" 特性）
pub fn replace_with_html(elements: &visdom::types::Elements, selector: &str, html: &str) -> Result<(), BoxDynError> {
    let mut target_elements = elements.find(selector);
    let mut new_elements = Vis::load(html)?;
    target_elements.replace_with(&mut new_elements);
    Ok(())
}

// 获取文本节点（需要启用 "text" 特性）
pub fn get_text_nodes<'a>(elements: &'a visdom::types::Elements<'a>, selector: &'a str) -> visdom::types::Texts<'a> {
    let found = elements.find(selector);
    found.texts(0) // 0 表示不限制递归深度
}

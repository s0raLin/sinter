use std::collections::HashMap;

pub struct Template<'a> {
    content: &'a str,
}

impl<'a> Template<'a> {
    pub fn new(content: &'a str) -> Self {
        Template { content }
    }

    pub fn replace(&self, key: &str, value: &str) -> TemplateOwned {
        let replaced = self.content.replace(&format!("{{{}}}", key), value);
        TemplateOwned { content: replaced }
    }

    pub fn replace_all(&self, replacements: &HashMap<&str, &str>) -> TemplateOwned {
        let mut content = self.content.to_string();
        for (key, value) in replacements {
            content = content.replace(&format!("{{{}}}", key), value);
        }
        TemplateOwned { content }
    }
}

pub struct TemplateOwned {
    content: String,
}

impl TemplateOwned {
    pub fn replace(&mut self, key: &str, value: &str) -> &mut Self {
        self.content = self.content.replace(&format!("{{{}}}", key), value);
        self
    }

    pub fn replace_all(&mut self, replacements: &HashMap<&str, &str>) -> &mut Self {
        for (key, value) in replacements {
            self.content = self.content.replace(&format!("{{{}}}", key), value);
        }
        self
    }

    pub fn as_str(&self) -> &str {
        &self.content
    }

    pub fn into_string(self) -> String {
        self.content
    }
}

impl<'a> From<&'a str> for Template<'a> {
    fn from(content: &'a str) -> Self {
        Template::new(content)
    }
}

impl From<String> for TemplateOwned {
    fn from(content: String) -> Self {
        TemplateOwned { content }
    }
}

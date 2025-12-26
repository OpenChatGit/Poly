//! Web development module for Poly
//! Provides HTML generation, components, routing, and state management

use std::collections::HashMap;

// ============================================
// Routing System
// ============================================

/// Route definition for SPA-like navigation
#[derive(Debug, Clone)]
pub struct Route {
    pub path: String,
    pub component: String,
    pub title: Option<String>,
}

/// Router for client-side navigation
#[derive(Debug, Clone)]
pub struct Router {
    pub routes: Vec<Route>,
    pub not_found: Option<String>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            not_found: None,
        }
    }

    pub fn route(mut self, path: &str, component: &str) -> Self {
        self.routes.push(Route {
            path: path.to_string(),
            component: component.to_string(),
            title: None,
        });
        self
    }

    pub fn route_with_title(mut self, path: &str, component: &str, title: &str) -> Self {
        self.routes.push(Route {
            path: path.to_string(),
            component: component.to_string(),
            title: Some(title.to_string()),
        });
        self
    }

    pub fn not_found(mut self, component: &str) -> Self {
        self.not_found = Some(component.to_string());
        self
    }

    /// Generate JavaScript router code
    pub fn render_js(&self) -> String {
        let mut routes_js = String::from("const routes = {\n");
        for route in &self.routes {
            routes_js.push_str(&format!(
                "  '{}': {{ component: '{}', title: {} }},\n",
                route.path,
                route.component,
                route.title.as_ref().map(|t| format!("'{}'", t)).unwrap_or("null".to_string())
            ));
        }
        routes_js.push_str("};\n\n");

        let not_found_js = match &self.not_found {
            Some(comp) => format!("const notFoundComponent = '{}';", comp),
            None => "const notFoundComponent = '<h1>404 - Not Found</h1>';".to_string(),
        };

        format!(r##"{}
{}

class Router {{
  constructor() {{
    this.currentPath = window.location.hash.slice(1) || '/';
    window.addEventListener('hashchange', () => this.navigate());
    window.addEventListener('load', () => this.navigate());
  }}

  navigate(path) {{
    if (path) {{
      window.location.hash = path;
      return;
    }}
    
    this.currentPath = window.location.hash.slice(1) || '/';
    const route = routes[this.currentPath];
    const app = document.getElementById('app');
    
    if (route) {{
      app.innerHTML = route.component;
      if (route.title) document.title = route.title;
    }} else {{
      app.innerHTML = notFoundComponent;
    }}
    
    // Dispatch route change event
    window.dispatchEvent(new CustomEvent('routechange', {{ 
      detail: {{ path: this.currentPath }} 
    }}));
  }}

  link(path, text, className = '') {{
    return '<a href="#' + path + '" class="' + className + '">' + text + '</a>';
  }}
}}

const router = new Router();
function navigate(path) {{ router.navigate(path); }}
function link(path, text, cls) {{ return router.link(path, text, cls); }}
"##, routes_js, not_found_js)
    }
}

impl Default for Router {
    fn default() -> Self { Self::new() }
}

// ============================================
// Component System
// ============================================

/// Reusable UI Component
#[derive(Debug, Clone)]
pub struct Component {
    pub name: String,
    pub props: Vec<String>,
    pub template: String,
    pub styles: Option<String>,
    pub script: Option<String>,
}

impl Component {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            props: Vec::new(),
            template: String::new(),
            styles: None,
            script: None,
        }
    }

    pub fn prop(mut self, name: &str) -> Self {
        self.props.push(name.to_string());
        self
    }

    pub fn props(mut self, names: &[&str]) -> Self {
        self.props.extend(names.iter().map(|s| s.to_string()));
        self
    }

    pub fn template(mut self, html: &str) -> Self {
        self.template = html.to_string();
        self
    }

    pub fn styles(mut self, css: &str) -> Self {
        self.styles = Some(css.to_string());
        self
    }

    pub fn script(mut self, js: &str) -> Self {
        self.script = Some(js.to_string());
        self
    }

    /// Render component definition as JavaScript
    pub fn render_js(&self) -> String {
        let props_list = self.props.join(", ");
        let template_escaped = self.template.replace('`', "\\`").replace("${", "\\${");
        
        format!(r#"
function {}({}) {{
  return `{}`;
}}
"#, self.name, props_list, template_escaped)
    }

    /// Render component with scoped styles
    pub fn render_with_styles(&self) -> String {
        let mut result = String::new();
        
        if let Some(css) = &self.styles {
            // Scope styles to component
            let scoped_css = css.replace(&self.name, &format!(".{}", self.name));
            result.push_str(&format!("<style>\n{}\n</style>\n", scoped_css));
        }
        
        result.push_str(&format!("<script>\n{}\n</script>", self.render_js()));
        
        if let Some(js) = &self.script {
            result.push_str(&format!("\n<script>\n{}\n</script>", js));
        }
        
        result
    }
}

// ============================================
// State Management
// ============================================

/// Reactive state store
#[derive(Debug, Clone)]
pub struct Store {
    pub name: String,
    pub initial_state: String,
    pub actions: Vec<(String, String)>, // (action_name, reducer_code)
}

impl Store {
    pub fn new(name: &str, initial_state: &str) -> Self {
        Self {
            name: name.to_string(),
            initial_state: initial_state.to_string(),
            actions: Vec::new(),
        }
    }

    pub fn action(mut self, name: &str, reducer: &str) -> Self {
        self.actions.push((name.to_string(), reducer.to_string()));
        self
    }

    /// Generate JavaScript store code
    pub fn render_js(&self) -> String {
        let mut actions_js = String::new();
        for (name, reducer) in &self.actions {
            actions_js.push_str(&format!(
                "  {}(payload) {{\n    {}\n    this._notify();\n  }}\n\n",
                name, reducer
            ));
        }

        format!(r#"
class {name}Store {{
  constructor() {{
    this.state = {initial};
    this._subscribers = [];
  }}

  getState() {{
    return this.state;
  }}

  subscribe(callback) {{
    this._subscribers.push(callback);
    return () => {{
      this._subscribers = this._subscribers.filter(cb => cb !== callback);
    }};
  }}

  _notify() {{
    this._subscribers.forEach(cb => cb(this.state));
  }}

{actions}}}

const {name_lower} = new {name}Store();
"#, 
            name = self.name,
            initial = self.initial_state,
            actions = actions_js,
            name_lower = self.name.to_lowercase()
        )
    }
}

// ============================================
// WebSocket Live Reload
// ============================================

/// Generate WebSocket-based live reload script
pub fn live_reload_script(port: u16) -> String {
    format!(r#"
(function() {{
  const ws = new WebSocket('ws://localhost:{}/ws');
  
  ws.onopen = () => console.log('[Poly] Live reload connected');
  
  ws.onmessage = (event) => {{
    const data = JSON.parse(event.data);
    if (data.type === 'reload') {{
      console.log('[Poly] Reloading...');
      window.location.reload();
    }} else if (data.type === 'css') {{
      // Hot reload CSS without full page refresh
      const links = document.querySelectorAll('link[rel="stylesheet"]');
      links.forEach(link => {{
        const href = link.href.split('?')[0];
        link.href = href + '?t=' + Date.now();
      }});
      console.log('[Poly] CSS updated');
    }} else if (data.type === 'update') {{
      // Partial update
      const el = document.querySelector(data.selector);
      if (el) el.innerHTML = data.html;
    }}
  }};
  
  ws.onclose = () => {{
    console.log('[Poly] Live reload disconnected, retrying...');
    setTimeout(() => window.location.reload(), 2000);
  }};
}})();
"#, port)
}

/// HTML Element builder
#[derive(Debug, Clone)]
pub struct Element {
    pub tag: String,
    pub attrs: HashMap<String, String>,
    pub children: Vec<Node>,
    pub self_closing: bool,
}

/// Node can be Element or Text
#[derive(Debug, Clone)]
pub enum Node {
    Element(Element),
    Text(String),
    Raw(String), // Raw HTML
}

impl Element {
    pub fn new(tag: &str) -> Self {
        let self_closing = matches!(tag, "img" | "br" | "hr" | "input" | "meta" | "link" | "area" | "base" | "col" | "embed" | "source" | "track" | "wbr");
        Self {
            tag: tag.to_string(),
            attrs: HashMap::new(),
            children: Vec::new(),
            self_closing,
        }
    }

    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.attrs.insert(key.to_string(), value.to_string());
        self
    }

    pub fn id(self, id: &str) -> Self {
        self.attr("id", id)
    }

    pub fn class(self, class: &str) -> Self {
        self.attr("class", class)
    }

    pub fn style(self, style: &str) -> Self {
        self.attr("style", style)
    }

    pub fn child(mut self, node: Node) -> Self {
        self.children.push(node);
        self
    }

    pub fn text(mut self, text: &str) -> Self {
        self.children.push(Node::Text(text.to_string()));
        self
    }

    pub fn children(mut self, nodes: Vec<Node>) -> Self {
        self.children.extend(nodes);
        self
    }

    pub fn render(&self) -> String {
        let mut html = String::new();
        
        // Opening tag
        html.push('<');
        html.push_str(&self.tag);
        
        // Attributes
        for (key, value) in &self.attrs {
            html.push(' ');
            html.push_str(key);
            html.push_str("=\"");
            html.push_str(&escape_html(value));
            html.push('"');
        }
        
        if self.self_closing {
            html.push_str(" />");
        } else {
            html.push('>');
            
            // Children
            for child in &self.children {
                html.push_str(&child.render());
            }
            
            // Closing tag
            html.push_str("</");
            html.push_str(&self.tag);
            html.push('>');
        }
        
        html
    }
}

impl Node {
    pub fn render(&self) -> String {
        match self {
            Node::Element(el) => el.render(),
            Node::Text(text) => escape_html(text),
            Node::Raw(html) => html.clone(),
        }
    }
}

/// Escape HTML special characters
pub fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// ============================================
// HTML Helper Functions
// ============================================

/// Create a div element
pub fn div() -> Element { Element::new("div") }

/// Create a span element
pub fn span() -> Element { Element::new("span") }

/// Create a paragraph element
pub fn p() -> Element { Element::new("p") }

/// Create a heading element
pub fn h1() -> Element { Element::new("h1") }
pub fn h2() -> Element { Element::new("h2") }
pub fn h3() -> Element { Element::new("h3") }
pub fn h4() -> Element { Element::new("h4") }
pub fn h5() -> Element { Element::new("h5") }
pub fn h6() -> Element { Element::new("h6") }

/// Create an anchor element
pub fn a(href: &str) -> Element { Element::new("a").attr("href", href) }

/// Create an image element
pub fn img(src: &str, alt: &str) -> Element { 
    Element::new("img").attr("src", src).attr("alt", alt) 
}

/// Create a button element
pub fn button() -> Element { Element::new("button") }

/// Create an input element
pub fn input(input_type: &str) -> Element { 
    Element::new("input").attr("type", input_type) 
}

/// Create a form element
pub fn form() -> Element { Element::new("form") }

/// Create an unordered list
pub fn ul() -> Element { Element::new("ul") }

/// Create an ordered list
pub fn ol() -> Element { Element::new("ol") }

/// Create a list item
pub fn li() -> Element { Element::new("li") }

/// Create a table element
pub fn table() -> Element { Element::new("table") }
pub fn tr() -> Element { Element::new("tr") }
pub fn td() -> Element { Element::new("td") }
pub fn th() -> Element { Element::new("th") }

/// Create a section element
pub fn section() -> Element { Element::new("section") }
pub fn article() -> Element { Element::new("article") }
pub fn header() -> Element { Element::new("header") }
pub fn footer() -> Element { Element::new("footer") }
pub fn nav() -> Element { Element::new("nav") }
pub fn main() -> Element { Element::new("main") }
pub fn aside() -> Element { Element::new("aside") }

/// Create a script element
pub fn script() -> Element { Element::new("script") }

/// Create a style element
pub fn style_tag() -> Element { Element::new("style") }

/// Create a link element (for CSS)
pub fn link(rel: &str, href: &str) -> Element {
    Element::new("link").attr("rel", rel).attr("href", href)
}

/// Create a meta element
pub fn meta(name: &str, content: &str) -> Element {
    Element::new("meta").attr("name", name).attr("content", content)
}

// ============================================
// HTML Document Builder
// ============================================

/// Complete HTML document builder
pub struct HtmlDocument {
    pub title: String,
    pub lang: String,
    pub head: Vec<Node>,
    pub body: Vec<Node>,
    pub styles: Vec<String>,
    pub scripts: Vec<String>,
}

impl HtmlDocument {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            lang: "en".to_string(),
            head: Vec::new(),
            body: Vec::new(),
            styles: Vec::new(),
            scripts: Vec::new(),
        }
    }

    pub fn lang(mut self, lang: &str) -> Self {
        self.lang = lang.to_string();
        self
    }

    pub fn head_element(mut self, node: Node) -> Self {
        self.head.push(node);
        self
    }

    pub fn body_element(mut self, node: Node) -> Self {
        self.body.push(node);
        self
    }

    pub fn style(mut self, css: &str) -> Self {
        self.styles.push(css.to_string());
        self
    }

    pub fn script(mut self, js: &str) -> Self {
        self.scripts.push(js.to_string());
        self
    }

    pub fn render(&self) -> String {
        let mut html = String::from("<!DOCTYPE html>\n");
        html.push_str(&format!("<html lang=\"{}\">\n", self.lang));
        
        // Head
        html.push_str("<head>\n");
        html.push_str("  <meta charset=\"UTF-8\">\n");
        html.push_str("  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
        html.push_str(&format!("  <title>{}</title>\n", escape_html(&self.title)));
        
        for node in &self.head {
            html.push_str("  ");
            html.push_str(&node.render());
            html.push('\n');
        }
        
        if !self.styles.is_empty() {
            html.push_str("  <style>\n");
            for style in &self.styles {
                html.push_str(style);
                html.push('\n');
            }
            html.push_str("  </style>\n");
        }
        
        html.push_str("</head>\n");
        
        // Body
        html.push_str("<body>\n");
        for node in &self.body {
            html.push_str("  ");
            html.push_str(&node.render());
            html.push('\n');
        }
        
        if !self.scripts.is_empty() {
            html.push_str("  <script>\n");
            for script in &self.scripts {
                html.push_str(script);
                html.push('\n');
            }
            html.push_str("  </script>\n");
        }
        
        html.push_str("</body>\n");
        html.push_str("</html>");
        
        html
    }
}

// ============================================
// CSS Builder
// ============================================

/// CSS Rule builder
pub struct CssRule {
    pub selector: String,
    pub properties: Vec<(String, String)>,
}

impl CssRule {
    pub fn new(selector: &str) -> Self {
        Self {
            selector: selector.to_string(),
            properties: Vec::new(),
        }
    }

    pub fn prop(mut self, property: &str, value: &str) -> Self {
        self.properties.push((property.to_string(), value.to_string()));
        self
    }

    pub fn render(&self) -> String {
        let mut css = format!("{} {{\n", self.selector);
        for (prop, val) in &self.properties {
            css.push_str(&format!("  {}: {};\n", prop, val));
        }
        css.push_str("}\n");
        css
    }
}

/// CSS Stylesheet builder
pub struct Stylesheet {
    pub rules: Vec<CssRule>,
}

impl Stylesheet {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn rule(mut self, rule: CssRule) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn render(&self) -> String {
        self.rules.iter().map(|r| r.render()).collect::<Vec<_>>().join("\n")
    }
}

impl Default for Stylesheet {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_render() {
        let el = div().class("container").text("Hello");
        assert_eq!(el.render(), "<div class=\"container\">Hello</div>");
    }

    #[test]
    fn test_nested_elements() {
        let el = div().class("card")
            .child(Node::Element(h1().text("Title")))
            .child(Node::Element(p().text("Content")));
        assert_eq!(el.render(), "<div class=\"card\"><h1>Title</h1><p>Content</p></div>");
    }

    #[test]
    fn test_self_closing() {
        let el = img("photo.jpg", "A photo");
        let html = el.render();
        assert!(html.contains("<img"));
        assert!(html.contains("src=\"photo.jpg\""));
        assert!(html.contains("alt=\"A photo\""));
        assert!(html.contains("/>"));
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<script>alert('xss')</script>"), 
                   "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;");
    }
}

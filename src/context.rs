use std::{borrow::Cow, collections::HashMap};

use crate::{
    parse::{self, Tokenize},
    value::Value,
    Object, Variable,
};

/// Context for expanding templates
///
/// ```
/// # use handybars::*;
/// let mut ctx = Context::new();
/// ctx.define(Variable::single("a"), "b");
/// assert_eq!(ctx.render("{{ a }}"), Ok("b".to_owned()));
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Context<'a> {
    vars: HashMap<Cow<'a, str>, Value<'a>>,
}
type Result<T, E = Error> = std::result::Result<T, E>;
impl std::error::Error for Error {}

/// Errors that may happen during rendering
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Forwarded from parsing
    Parse(parse::Error),
    /// Tried to expand a template variable that we don't have a value for
    MissingVariable(Variable<'static>),
    /// Tried to expand an object template variable
    TriedToExpandObject(Variable<'static>),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Parse(p) => f.write_fmt(format_args!("parse: {p}")),
            Error::MissingVariable(var) => {
                f.write_fmt(format_args!("missing variable in template: '{var}'"))
            }
            Error::TriedToExpandObject(var) => {
                f.write_fmt(format_args!("tried to expand object variable: '{var}'"))
            }
        }
    }
}

impl From<parse::Error> for Error {
    fn from(value: parse::Error) -> Self {
        Self::Parse(value)
    }
}
macro_rules! force_object {
    ($entry:expr) => {
        $entry
            .and_modify(|o| match o {
                Value::String(_) => *o = Object::new().into(),
                Value::Object(_) => {}
            })
            .or_insert(Object::new().into())
            .as_object_mut()
            .unwrap()
    };
}

impl<'a> Context<'a> {
    /// Create a new context with no variables defined
    pub fn new() -> Self {
        Self::default()
    }
    /// Map a variable to a value for template expansion
    ///
    /// Values are merged into existing definitions where possible
    ///
    /// ```
    /// # use handybars::{Context, Object, Variable};
    /// # use std::str::FromStr;
    /// let mut obj = Object::new().with_property("b", "c");
    /// let mut ctx = Context::new().with_define("a".parse().unwrap(), obj.clone());
    ///
    /// ctx.define("a.d".parse().unwrap(), "f");
    /// obj.add_property("d", "f");
    /// assert_eq!(ctx.get_value(&Variable::single("a")), Some(&obj.clone().into()));
    ///
    /// ctx.define("a.d.e".parse().unwrap(), Object::new().with_property("x", "y"));
    /// obj.add_property("d", Object::new().with_property("e", Object::new().with_property("x", "y")));
    /// assert_eq!(ctx.get_value(&Variable::single("a")), Some(&obj.clone().into()));
    /// ```
    ///
    /// Note that `f` is overwritten here as `e` is an object and so the value pointed to by `d` must also
    /// change to an object.
    ///
    pub fn define(&mut self, var: Variable<'a>, value: impl Into<Value<'a>>) -> &mut Self {
        match var.inner {
            crate::VariableInner::Segments(mut segs) => {
                let mut parent = force_object!(self.vars.entry(segs[0].clone()));
                let last = segs.pop().unwrap();
                for level in segs.into_iter().skip(1) {
                    parent = force_object!(parent.values.entry(level));
                }
                parent.add_property(last, value);
            }
            crate::VariableInner::Single(s) => {
                let mut value = Some(value);
                self.vars
                    .entry(s.clone())
                    .and_modify(|o| match o {
                        Value::String(_) => {
                            *o = value.take().unwrap().into();
                        }
                        Value::Object(o) => {
                            o.add_property(s, value.take().unwrap());
                        }
                    })
                    .or_insert_with(|| value.take().unwrap().into());
            }
        }
        self
    }
    /// Builder version of [`define`](Context::define)
    pub fn with_define(mut self, var: Variable<'a>, value: impl Into<Value<'a>>) -> Self {
        self.define(var, value);
        self
    }
    /// Resolve a variable added with [`define`](Context::define)
    ///
    /// This will "drill down" to the lowest point it can get
    ///
    /// ```
    /// # use handybars::{Context, Object, Value};
    /// # use std::str::FromStr;
    /// let ctx = Context::new()
    ///     .with_define("a".parse().unwrap(),
    ///         Object::new()
    ///             .with_property("x", "y")
    ///             .with_property("b", Object::new().with_property("c", "d"))
    ///     );
    ///
    /// assert_eq!(
    ///     ctx.get_value(&"a.b".parse().unwrap()),
    ///     Some(&Value::Object(Object::new().with_property("c", "d")))
    /// );
    /// assert_eq!(
    ///     ctx.get_value(&"a.b.c".parse().unwrap()),
    ///     Some(&Value::String("d".into()))
    /// );
    ///
    /// assert_eq!(
    ///     ctx.get_value(&"a.x".parse().unwrap()),
    ///     Some(&Value::String("y".into()))
    /// );
    /// ```
    pub fn get_value(&self, var: &Variable<'a>) -> Option<&Value<'a>> {
        match &var.inner {
            crate::VariableInner::Segments(segs) => {
                let mut parent = self.vars.get(&segs[0])?;
                let last = segs.last().unwrap();
                for level in segs.iter().skip(1).take(segs.len() - 2) {
                    parent = parent.as_object()?.property(level)?;
                }
                parent.as_object()?.property(last)
            }
            crate::VariableInner::Single(s) => self.vars.get(s),
        }
    }
    /// Expand a single variable
    ///
    /// ```
    /// # use handybars::{Context, Variable};
    /// let var = Variable::single("a");
    /// let ctx = Context::new().with_define(var.clone(), "b");
    /// assert_eq!(ctx.expand(&var), Ok("b".to_owned()));
    /// ```
    pub fn expand(&self, var: &Variable<'a>) -> Result<String> {
        let val = self
            .get_value(var)
            .ok_or_else(|| Error::MissingVariable(var.clone().into_owned()))?;
        if val.is_object() {
            Err(Error::TriedToExpandObject(var.clone().into_owned()))
        } else {
            Ok(val.as_string().unwrap().clone().into_owned())
        }
    }

    /// Render a template
    ///
    /// ```
    /// # use handybars::{Context, Variable};
    /// let var = Variable::single("a");
    /// let ctx = Context::new().with_define(var.clone(), "b");
    /// assert_eq!(ctx.render("some text {{ a }}"), Ok("some text b".to_owned()));
    /// ```
    pub fn render<'b>(&self, input: &'b str) -> Result<String> {
        let mut output = String::new();
        for token in Tokenize::<'b>::new(input) {
            let token = token?;
            match token {
                parse::Token::Variable(v) => output.push_str(&self.expand(&v)?),
                parse::Token::Str(s) => {
                    output.push_str(s);
                }
            }
        }
        Ok(output)
    }
    /// Append another `Context`'s variables
    ///
    /// This operates in place, see [`merge`](Context::merge) for a streamable version
    ///
    /// ```
    /// # use handybars::{Context, Variable, Value};
    /// let mut ctx = Context::new();
    /// ctx.append(Context::new().with_define(Variable::single("a"), "b"));
    /// assert_eq!(ctx.get_value(&Variable::single("a")), Some(&Value::String("b".into())));
    /// ```
    pub fn append(&mut self, other: Self) -> &mut Self {
        self.vars.extend(other.vars);
        self
    }
    /// Stream version of `append`
    pub fn merge(mut self, other: Self) -> Self {
        self.append(other);
        self
    }
}

impl<'a> Extend<(Variable<'a>, Value<'a>)> for Context<'a> {
    /// Extend a `Context` with an iterator of defines
    ///
    /// ```
    /// # use handybars::{Context, Variable, Value};
    /// let mut ctx = Context::new();
    /// ctx.extend([(Variable::single("a"), Value::String("b".into()))].into_iter());
    /// assert_eq!(ctx.get_value(&Variable::single("a")), Some(&Value::String("b".into())));
    /// ```
    fn extend<T: IntoIterator<Item = (Variable<'a>, Value<'a>)>>(&mut self, iter: T) {
        for (var, val) in iter {
            self.define(var, val);
        }
    }
}
impl<'a> FromIterator<(Variable<'a>, Value<'a>)> for Context<'a> {
    fn from_iter<T: IntoIterator<Item = (Variable<'a>, Value<'a>)>>(iter: T) -> Self {
        let mut me = Self::default();
        me.extend(iter);
        me
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Object;

    #[test]
    fn defining_an_object_variable_creates_path() {
        let mut ctx = Context::new();
        ctx.define(
            Variable::single("hello"),
            Object::new().with_property("world", Object::new().with_property("test", "val")),
        );
        assert_eq!(ctx.render("{{hello.world.test}}"), Ok("val".to_owned()));
    }

    #[test]
    fn context_can_register_single_variables() {
        let mut ctx = Context::new();
        ctx.define(
            Variable::single("hello"),
            Object::new().with_property("world", "sup"),
        );
    }
    #[test]
    fn context_renders_templates_using_defines() {
        let mut ctx = Context::new();
        ctx.define(Variable::single("t1"), "value");
        let expanded = ctx.render("{{ t1 }}").unwrap();
        assert_eq!(expanded, "value");
        assert_eq!(
            ctx.render("{{ notexist }}"),
            Err(Error::MissingVariable(Variable::single("notexist"))),
            "missing defines cause an error"
        );
    }
    #[test]
    fn trying_to_expand_an_object_variable_is_an_error() {
        let ctx = Context::new().with_define("a".parse().unwrap(), Object::new());
        assert_eq!(
            ctx.expand(&"a".parse().unwrap()),
            Err(Error::TriedToExpandObject("a".parse().unwrap()))
        );
    }
    #[test]
    fn from_iterator_for_context_adds_defines_for_each_element() {
        let ctx: Context = [
            (Variable::single("a"), Value::String("b".into())),
            (Variable::single("b"), Value::String("c".into())),
        ]
        .into_iter()
        .collect();
        assert_eq!(ctx.render("{{a}}"), Ok("b".to_owned()));
        assert_eq!(ctx.render("{{b}}"), Ok("c".to_owned()));
    }
    #[test]
    fn double_defintition_should_overwrite() {
        let ctx = Context::new()
        .with_define(Variable::single("a"), "b")
        .with_define(Variable::single("a"), "c");
        assert_eq!(ctx.render("{{a}}"), Ok("c".to_owned()));
    }
}

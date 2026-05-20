use crate::builtins::unicode_data;
use crate::host::HostFunction;
use crate::object::object::JSObject;
use crate::runtime::context::JSContext;
use crate::value::JSValue;

pub fn init_regexp(ctx: &mut JSContext) {
    ctx.register_builtin(
        "regexp_constructor",
        HostFunction::new("RegExp", 2, regexp_constructor),
    );

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        let regexp_atom = ctx.common_atoms.regexp;

        let regexp_func = create_builtin_function(ctx, "regexp_constructor");
        global_obj.set(regexp_atom, regexp_func);
    }

    let proto_atom = ctx.intern("RegExpPrototype");
    let mut proto_obj = JSObject::new();

    proto_obj.set(
        ctx.common_atoms.test,
        create_builtin_function(ctx, "regexp_test"),
    );
    proto_obj.set(
        ctx.common_atoms.exec,
        create_builtin_function(ctx, "regexp_exec"),
    );
    proto_obj.set(
        ctx.common_atoms.to_string,
        create_builtin_function(ctx, "regexp_toString"),
    );
    proto_obj.set(
        ctx.intern("compile"),
        create_builtin_function(ctx, "regexp_compile"),
    );
    proto_obj.set(
        ctx.common_atoms.source,
        JSValue::new_string(ctx.intern("(?:)")),
    );
    proto_obj.set(ctx.common_atoms.global, JSValue::bool(false));
    proto_obj.set(ctx.common_atoms.ignore_case, JSValue::bool(false));
    proto_obj.set(ctx.common_atoms.multiline, JSValue::bool(false));
    proto_obj.set(ctx.common_atoms.sticky, JSValue::bool(false));
    proto_obj.set(ctx.common_atoms.unicode, JSValue::bool(false));
    proto_obj.set(ctx.intern("unicodeSets"), JSValue::bool(false));
    proto_obj.set(ctx.intern("dotAll"), JSValue::bool(false));
    proto_obj.set(ctx.intern("lastIndex"), JSValue::new_int(0));

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        proto_obj.prototype = Some(obj_proto_ptr);
    }

    let proto_ptr = Box::into_raw(Box::new(proto_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);

    ctx.set_regexp_prototype(proto_ptr);
    let proto_value = JSValue::new_object(proto_ptr);

    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(proto_atom, proto_value);
    }
}

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let mut func = crate::object::function::JSFunction::new_builtin(ctx.intern(name), 1);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

pub fn regexp_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let pattern = if args.is_empty() {
        String::new()
    } else if args[0].is_string() {
        ctx.get_atom_str(args[0].get_atom()).to_string()
    } else if args[0].is_object() {
        return args[0].clone();
    } else {
        String::new()
    };

    let flags = if args.len() > 1 && args[1].is_string() {
        ctx.get_atom_str(args[1].get_atom()).to_string()
    } else {
        String::new()
    };

    create_regexp_object(ctx, &pattern, &flags)
}

fn create_regexp_object(ctx: &mut JSContext, pattern: &str, flags: &str) -> JSValue {
    let mut regexp_obj = JSObject::new();

    if let Some(proto_ptr) = ctx.get_regexp_prototype() {
        regexp_obj.prototype = Some(proto_ptr);
    }

    regexp_obj.set(
        ctx.common_atoms.source,
        JSValue::new_string(ctx.intern(pattern)),
    );
    regexp_obj.set(ctx.intern("lastIndex"), JSValue::new_int(0));

    let global = flags.contains('g');
    let ignore_case = flags.contains('i');
    let multiline = flags.contains('m');
    let sticky = flags.contains('y');
    let unicode = flags.contains('u');
    let dot_all = flags.contains('s');
    let unicode_sets = flags.contains('v');

    regexp_obj.set(ctx.common_atoms.global, JSValue::bool(global));
    regexp_obj.set(ctx.common_atoms.ignore_case, JSValue::bool(ignore_case));
    regexp_obj.set(ctx.common_atoms.multiline, JSValue::bool(multiline));
    regexp_obj.set(ctx.common_atoms.sticky, JSValue::bool(sticky));
    regexp_obj.set(ctx.common_atoms.unicode, JSValue::bool(unicode));
    regexp_obj.set(ctx.intern("unicodeSets"), JSValue::bool(unicode_sets));
    regexp_obj.set(ctx.intern("dotAll"), JSValue::bool(dot_all));

    regexp_obj.set(
        ctx.common_atoms.__pattern__,
        JSValue::new_string(ctx.intern(pattern)),
    );
    regexp_obj.set(
        ctx.common_atoms.__flags__,
        JSValue::new_string(ctx.intern(flags)),
    );

    let mut flags = crate::regexp::RegexFlags::default();
    flags.ignore_case = ignore_case;
    flags.global = global;
    flags.multi_line = multiline;
    flags.dot_all = dot_all;
    flags.unicode = unicode;
    flags.sticky = sticky;
    if let Ok(re) = crate::regexp::Regex::new_with_flags(pattern, flags) {
        regexp_obj.set_compiled_regex(re);
    }

    let ptr = Box::into_raw(Box::new(regexp_obj)) as usize;
    JSValue::new_object(ptr)
}

pub fn create_regexp_object_precompiled(
    ctx: &mut JSContext,
    pattern: &str,
    flags: &str,
    compiled_re: crate::regexp::Regex,
) -> JSValue {
    use crate::object::object::JSObject;
    let mut regexp_obj = JSObject::new();

    if let Some(proto_ptr) = ctx.get_regexp_prototype() {
        regexp_obj.prototype = Some(proto_ptr);
    }

    regexp_obj.set(
        ctx.common_atoms.source,
        JSValue::new_string(ctx.intern(pattern)),
    );
    regexp_obj.set(ctx.intern("lastIndex"), JSValue::new_int(0));

    let global = flags.contains('g');
    let ignore_case = flags.contains('i');
    let multiline = flags.contains('m');
    let sticky = flags.contains('y');
    let unicode = flags.contains('u');
    let dot_all = flags.contains('s');
    let unicode_sets = flags.contains('v');

    regexp_obj.set(ctx.common_atoms.global, JSValue::bool(global));
    regexp_obj.set(ctx.common_atoms.ignore_case, JSValue::bool(ignore_case));
    regexp_obj.set(ctx.common_atoms.multiline, JSValue::bool(multiline));
    regexp_obj.set(ctx.common_atoms.sticky, JSValue::bool(sticky));
    regexp_obj.set(ctx.common_atoms.unicode, JSValue::bool(unicode));
    regexp_obj.set(ctx.intern("unicodeSets"), JSValue::bool(unicode_sets));
    regexp_obj.set(ctx.intern("dotAll"), JSValue::bool(dot_all));

    regexp_obj.set(
        ctx.common_atoms.__pattern__,
        JSValue::new_string(ctx.intern(pattern)),
    );
    regexp_obj.set(
        ctx.common_atoms.__flags__,
        JSValue::new_string(ctx.intern(flags)),
    );

    regexp_obj.set_compiled_regex(compiled_re);

    let ptr = Box::into_raw(Box::new(regexp_obj)) as usize;
    JSValue::new_object(ptr)
}

pub fn regexp_test(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::bool(false);
    }

    let this = &args[0];
    let test_str = if args.len() > 1 && args[1].is_string() {
        ctx.get_atom_str(args[1].get_atom()).to_string()
    } else {
        String::new()
    };

    if !this.is_object() {
        return JSValue::bool(false);
    }

    let regexp_obj = this.as_object_mut();

    let pattern_atom = ctx.common_atoms.__pattern__;
    let flags_atom = ctx.common_atoms.__flags__;
    let last_index_atom = ctx.intern("lastIndex");

    let pattern = if let Some(p) = regexp_obj.get(pattern_atom) {
        if p.is_string() {
            ctx.get_atom_str(p.get_atom()).to_string()
        } else {
            return JSValue::bool(false);
        }
    } else {
        return JSValue::bool(false);
    };

    let flags = if let Some(f) = regexp_obj.get(flags_atom) {
        if f.is_string() {
            ctx.get_atom_str(f.get_atom()).to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let last_index = if let Some(li) = regexp_obj.get(last_index_atom) {
        if li.is_int() {
            li.get_int() as usize
        } else {
            0
        }
    } else {
        0
    };

    let ignore_case = flags.contains('i');
    let global = flags.contains('g');
    let unicode_sets = flags.contains('v');

    let result = if let Some(re) = regexp_obj.get_compiled_regex() {
        match_regex_precompiled(re, &test_str, last_index)
    } else {
        match_pattern_with_len(&pattern, &test_str, last_index, ignore_case, unicode_sets)
    };

    if let Some((match_pos, match_len)) = result {
        if global {
            let new_last_index = (match_pos + match_len) as i64;
            regexp_obj.set(last_index_atom, JSValue::new_int(new_last_index));
        }
        JSValue::bool(true)
    } else {
        regexp_obj.set(last_index_atom, JSValue::new_int(0));
        JSValue::bool(false)
    }
}

pub fn regexp_exec(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::null();
    }

    let this = &args[0];
    let test_str = if args.len() > 1 && args[1].is_string() {
        ctx.get_atom_str(args[1].get_atom()).to_string()
    } else {
        String::new()
    };

    if !this.is_object() {
        return JSValue::null();
    }

    let regexp_obj = this.as_object_mut();

    let pattern_atom = ctx.common_atoms.__pattern__;
    let flags_atom = ctx.common_atoms.__flags__;
    let last_index_atom = ctx.intern("lastIndex");

    let pattern = if let Some(p) = regexp_obj.get(pattern_atom) {
        if p.is_string() {
            ctx.get_atom_str(p.get_atom()).to_string()
        } else {
            return JSValue::null();
        }
    } else {
        return JSValue::null();
    };

    let flags = if let Some(f) = regexp_obj.get(flags_atom) {
        if f.is_string() {
            ctx.get_atom_str(f.get_atom()).to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let last_index = if let Some(li) = regexp_obj.get(last_index_atom) {
        if li.is_int() {
            li.get_int() as usize
        } else {
            0
        }
    } else {
        0
    };

    let ignore_case = flags.contains('i');
    let global = flags.contains('g');
    let unicode_sets = flags.contains('v');

    let result = if let Some(re) = regexp_obj.get_compiled_regex() {
        match_regex_precompiled(re, &test_str, last_index)
    } else {
        match_pattern_with_len(&pattern, &test_str, last_index, ignore_case, unicode_sets)
    };

    if let Some((match_pos, match_len)) = result {
        let match_end = (match_pos + match_len).min(test_str.len());
        let match_str = &test_str[match_pos..match_end];

        if global {
            regexp_obj.set(last_index_atom, JSValue::new_int(match_end as i64));
        }

        let mut result_array = JSObject::new_array();
        let zero_atom = ctx.intern("0");
        let index_atom = ctx.common_atoms.index;
        let input_atom = ctx.common_atoms.input;
        let length_atom = ctx.common_atoms.length;

        result_array.set(zero_atom, JSValue::new_string(ctx.intern(match_str)));
        result_array.set(index_atom, JSValue::new_int(match_pos as i64));
        result_array.set(input_atom, JSValue::new_string(ctx.intern(&test_str)));
        result_array.set(length_atom, JSValue::new_int(1));

        let result_ptr = Box::into_raw(Box::new(result_array)) as usize;
        JSValue::new_object(result_ptr)
    } else {
        regexp_obj.set(last_index_atom, JSValue::new_int(0));
        JSValue::null()
    }
}

fn match_regex_precompiled(
    re: &crate::regexp::Regex,
    text: &str,
    start: usize,
) -> Option<(usize, usize)> {
    let search_from = start.min(text.len());
    if let Some(m) = re.find(&text[search_from..]) {
        return Some((search_from + m.start(), m.end() - m.start()));
    }
    None
}

fn match_pattern_with_len(
    pattern: &str,
    text: &str,
    start: usize,
    ignore_case: bool,
    unicode_sets: bool,
) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return Some((0, 0));
    }

    if unicode_sets && pattern.contains("\\p{") {
        return match_pattern_unicode_sets(pattern, text, start, ignore_case).map(|pos| (pos, 0));
    }

    if !pattern.contains('\\')
        && !pattern.contains('.')
        && !pattern.contains('*')
        && !pattern.contains('+')
        && !pattern.contains('?')
        && !pattern.contains('[')
        && !pattern.contains('(')
        && !pattern.contains('{')
        && !pattern.contains('|')
        && !pattern.contains('^')
        && !pattern.contains('$')
    {
        let search_text = if ignore_case {
            text.to_lowercase()
        } else {
            text.to_string()
        };
        let search_pattern = if ignore_case {
            pattern.to_lowercase()
        } else {
            pattern.to_string()
        };
        let search_from = start.min(search_text.len());
        if let Some(pos) = search_text[search_from..].find(&search_pattern) {
            return Some((search_from + pos, search_pattern.len()));
        }
        return None;
    }

    let mut flags = crate::regexp::RegexFlags::default();
    flags.ignore_case = ignore_case;
    let compiled = crate::regexp::Regex::new_with_flags(pattern, flags);

    if let Ok(re) = compiled {
        let search_from = start.min(text.len());
        if let Some(m) = re.find(&text[search_from..]) {
            return Some((search_from + m.start(), m.end() - m.start()));
        }
        return None;
    }

    let search_text = if ignore_case {
        text.to_lowercase()
    } else {
        text.to_string()
    };

    let search_pattern = if ignore_case {
        pattern.to_lowercase()
    } else {
        pattern.to_string()
    };

    if start <= search_text.len() {
        let max_start = if search_pattern.len() <= search_text.len() {
            search_text.len() - search_pattern.len()
        } else {
            return None;
        };

        for i in start..=max_start {
            if i + search_pattern.len() <= search_text.len() {
                let slice = &search_text[i..i + search_pattern.len()];
                if slice == search_pattern {
                    return Some((i, search_pattern.len()));
                }
            }
        }
    }

    None
}

fn match_pattern(
    pattern: &str,
    text: &str,
    start: usize,
    ignore_case: bool,
    unicode_sets: bool,
) -> Option<usize> {
    match_pattern_with_len(pattern, text, start, ignore_case, unicode_sets).map(|(pos, _)| pos)
}

fn match_pattern_unicode_sets(
    pattern: &str,
    text: &str,
    start: usize,
    ignore_case: bool,
) -> Option<usize> {
    let char_class = match parse_unicode_set_pattern(pattern) {
        Ok(cc) => cc,
        Err(_) => return None,
    };

    let search_text: Vec<char> = if ignore_case {
        text.to_lowercase().chars().collect()
    } else {
        text.chars().collect()
    };

    let max_start = if start < search_text.len() {
        start
    } else {
        return None;
    };

    for i in max_start..search_text.len() {
        let ch = search_text[i];
        if char_class.matches(ch) {
            return Some(i);
        }
    }

    None
}

#[derive(Debug, Clone)]
enum CharClass {
    Char(char),

    UnicodeSet(UnicodeSetKind),

    Negated(Box<CharClass>),

    Difference(Box<CharClass>, Box<CharClass>),

    Union(Box<CharClass>, Box<CharClass>),

    Intersection(Box<CharClass>, Box<CharClass>),

    Range(char, char),

    Empty,
}

#[derive(Debug, Clone)]
enum UnicodeSetKind {
    UppercaseLetter,
    LowercaseLetter,
    DecimalNumber,
    Letter,
    Mark,
    Separator,
    Other,
}

impl CharClass {
    fn matches(&self, ch: char) -> bool {
        match self {
            CharClass::Char(c) => *c == ch,
            CharClass::UnicodeSet(kind) => kind.matches(ch),
            CharClass::Negated(inner) => !inner.matches(ch),
            CharClass::Difference(a, b) => a.matches(ch) && !b.matches(ch),
            CharClass::Union(a, b) => a.matches(ch) || b.matches(ch),
            CharClass::Intersection(a, _b) => a.matches(ch),
            CharClass::Range(start, end) => *start <= ch && ch <= *end,
            CharClass::Empty => false,
        }
    }
}

impl UnicodeSetKind {
    fn matches(&self, ch: char) -> bool {
        let cp = ch as u32;
        match self {
            UnicodeSetKind::UppercaseLetter => unicode_data::GC_UPPERCASE_LETTER.contains(cp),
            UnicodeSetKind::LowercaseLetter => unicode_data::GC_LOWERCASE_LETTER.contains(cp),
            UnicodeSetKind::DecimalNumber => unicode_data::GC_DECIMAL_NUMBER.contains(cp),
            UnicodeSetKind::Letter => {
                unicode_data::GC_UPPERCASE_LETTER.contains(cp)
                    || unicode_data::GC_LOWERCASE_LETTER.contains(cp)
            }
            UnicodeSetKind::Mark => {
                (0x0300..=0x036F).contains(&cp) || (0x1DC0..=0x1DFF).contains(&cp)
            }
            UnicodeSetKind::Separator => {
                cp == 0x0020
                    || cp == 0x00A0
                    || cp == 0x1680
                    || (0x2000..=0x200A).contains(&cp)
                    || cp == 0x202F
                    || cp == 0x205F
                    || cp == 0x3000
            }
            UnicodeSetKind::Other => true,
        }
    }
}

fn parse_unicode_set_pattern(pattern: &str) -> Result<CharClass, ()> {
    if let Some(class_start) = pattern.find('[') {
        let class_end = pattern.rfind(']').ok_or(())?;
        if class_start < class_end {
            let class_content = &pattern[class_start + 1..class_end];
            return parse_character_class(class_content);
        }
    }

    if pattern.starts_with("\\p{") {
        let end = pattern.find('}').ok_or(())?;
        let prop_content = &pattern[3..end];
        return parse_unicode_property(&prop_content);
    }

    Ok(CharClass::Empty)
}

fn parse_character_class(content: &str) -> Result<CharClass, ()> {
    let content = content.trim();

    let negated = content.starts_with('^');
    let content = if negated { &content[1..] } else { content };

    if let Some(diff_pos) = content.find("--") {
        let left = &content[..diff_pos];
        let right = &content[diff_pos + 2..];
        let left_class = parse_class_element(left)?;
        let right_class = parse_class_element(right)?;
        let result = CharClass::Difference(Box::new(left_class), Box::new(right_class));
        return if negated {
            Ok(CharClass::Negated(Box::new(result)))
        } else {
            Ok(result)
        };
    }

    if let Some(union_pos) = content.find("||") {
        let left = &content[..union_pos];
        let right = &content[union_pos + 2..];
        let left_class = parse_class_element(left)?;
        let right_class = parse_class_element(right)?;
        let result = CharClass::Union(Box::new(left_class), Box::new(right_class));
        return if negated {
            Ok(CharClass::Negated(Box::new(result)))
        } else {
            Ok(result)
        };
    }

    if let Some(inter_pos) = content.find("&&") {
        let left = &content[..inter_pos];
        let right = &content[inter_pos + 2..];
        let left_class = parse_class_element(left)?;
        let right_class = parse_class_element(right)?;
        let result = CharClass::Intersection(Box::new(left_class), Box::new(right_class));
        return if negated {
            Ok(CharClass::Negated(Box::new(result)))
        } else {
            Ok(result)
        };
    }

    let elem = parse_class_element(content)?;
    if negated {
        Ok(CharClass::Negated(Box::new(elem)))
    } else {
        Ok(elem)
    }
}

fn parse_class_element(content: &str) -> Result<CharClass, ()> {
    let content = content.trim();

    if content.starts_with("\\p{") {
        let end = content.find('}').ok_or(())?;
        let prop_content = &content[3..end];
        parse_unicode_property(prop_content)
    } else if content.starts_with("\\P{") {
        let end = content.find('}').ok_or(())?;
        let prop_content = &content[3..end];
        let prop = parse_unicode_property(prop_content)?;
        Ok(CharClass::Negated(Box::new(prop)))
    } else if content.len() >= 3 && content.chars().nth(1) == Some('-') {
        let start = content.chars().next().ok_or(())?;
        let end = content.chars().last().ok_or(())?;
        Ok(CharClass::Range(start, end))
    } else if content.len() == 1 {
        Ok(CharClass::Char(content.chars().next().ok_or(())?))
    } else {
        Err(())
    }
}

fn parse_unicode_property(prop_content: &str) -> Result<CharClass, ()> {
    let prop_content = prop_content.trim();

    let (prop, value) = if let Some(eq_pos) = prop_content.find('=') {
        (&prop_content[..eq_pos], &prop_content[eq_pos + 1..])
    } else {
        ("General_Category", prop_content)
    };

    let prop = prop.trim();
    let value = value.trim();

    if prop == "General_Category" || prop == "gc" {
        match value {
            "Lu" | "Uppercase_Letter" => Ok(CharClass::UnicodeSet(UnicodeSetKind::UppercaseLetter)),
            "Ll" | "Lowercase_Letter" => Ok(CharClass::UnicodeSet(UnicodeSetKind::LowercaseLetter)),
            "Nd" | "Decimal_Number" | "Digit" => {
                Ok(CharClass::UnicodeSet(UnicodeSetKind::DecimalNumber))
            }
            "L" | "Letter" => Ok(CharClass::UnicodeSet(UnicodeSetKind::Letter)),
            "M" | "Mark" => Ok(CharClass::UnicodeSet(UnicodeSetKind::Mark)),
            "Z" | "Separator" => Ok(CharClass::UnicodeSet(UnicodeSetKind::Separator)),
            _ => Ok(CharClass::UnicodeSet(UnicodeSetKind::Other)),
        }
    } else {
        Ok(CharClass::UnicodeSet(UnicodeSetKind::Other))
    }
}

pub fn regexp_to_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern("/(?:)/"));
    }

    let this = &args[0];
    if !this.is_object() {
        return JSValue::new_string(ctx.intern("/(?:)/"));
    }

    let regexp_obj = this.as_object();

    let source_atom = ctx.common_atoms.source;
    let flags_atom = ctx.common_atoms.__flags__;

    let source = if let Some(s) = regexp_obj.get(source_atom) {
        if s.is_string() {
            ctx.get_atom_str(s.get_atom()).to_string()
        } else {
            "(?:)".to_string()
        }
    } else {
        "(?:)".to_string()
    };

    let flags = if let Some(f) = regexp_obj.get(flags_atom) {
        if f.is_string() {
            ctx.get_atom_str(f.get_atom()).to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    JSValue::new_string(ctx.intern(&format!("/{}/{}", source, flags)))
}

pub fn regexp_compile(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    regexp_constructor(ctx, args)
}

pub fn matches(pattern: &str, text: &str, ignore_case: bool) -> bool {
    match_pattern(pattern, text, 0, ignore_case, false).is_some()
}

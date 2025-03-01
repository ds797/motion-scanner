use regex::Regex;

pub fn snake_to_upper_camel(s: &str) -> String {
	s.split('_').map(|w| {
		let mut chars = w.chars();
		match chars.next() {
			Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
			None => String::new(),
		}
	}).collect()
}

pub fn sanitize_enum_entry_name(name: String) -> String {
	let mut chars = name.chars();

	let first = chars.next().expect("Enum entry has no name!");
	if first.is_ascii_alphabetic() || first == '_' {
		name.to_string()
	} else {
		format!("N{}", name)
	}		
}

pub fn format_documentation(text: &str) -> anyhow::Result<String> {
	let regex = Regex::new(" {2,}")?;
	let text = regex.replace_all(text, " ");
	let text = text.replace("\t", "");

	Ok(text.to_string())
}

pub fn to_title(text: &String) -> String {
	return text[0..1].to_ascii_uppercase() + &text[1..text.len()]
}
use quick_xml::events::BytesStart;

pub fn parse_attributes(event: &BytesStart<'_>) -> anyhow::Result<Vec<(String, String)>> {
	let mut attrs: Vec<(String, String)> = vec![];
	let attributes = event.html_attributes();

	for attribute in attributes {
		let attribute = attribute?;

		let key = attribute.key.into_inner();
		let value = attribute.value;

		attrs.push((String::from_utf8(key.to_vec())?, String::from_utf8(value.to_vec())?));
	}

	Ok(attrs)
}

pub fn find_attr(attributes: &Vec<(String, String)>, name: &str) -> String {
	let attr = attributes.iter().find(|a| a.0 == name).map(|a| a.1.clone());
	attr.expect(format!("No attribute with name {} found", name).as_str())
}

pub fn try_find_attr(attributes: &Vec<(String, String)>, name: &str) -> Option<String> {
	attributes.iter().find(|a| a.0 == name).map(|a| a.1.clone())
}
use core::str;
use std::{fs::File, io::Write};
use std::io::BufReader;
use quick_xml::{
	reader::Reader,
	events::Event::{
		Start,
		End,
		Eof,
		Empty,
		Text,
		CData,
		Comment,
		Decl,
		PI,
		DocType,
	},
};

mod wl_types;
mod convert;
mod format;
mod xml;

use regex::Regex;
use wl_types::{
	Interface,
	Description,
	RequestEvent,
	Arg,
	EnumEntry,
	Enum,
};

enum ElementType {
	Interface(Interface),
	ReqEvent(RequestEvent),
	Enum(Enum),
	Description(Description),
}

struct Stack {
	elements: Vec<ElementType>,
	interfaces: Vec<Interface>,
}

impl Stack {
	fn last(&mut self) -> &mut ElementType {
		self.elements.last_mut().expect("No last element")
	}
	fn last_interface(&mut self) -> &mut Interface {
		self.elements.iter_mut().rev().find_map(|e| match e {
			ElementType::Interface(i) => Some(i),
			_ => None,
		}).expect("No last interface")
	}
	fn last_reqevent(&mut self) -> &mut RequestEvent {
		self.elements.iter_mut().rev().find_map(|e| match e {
			ElementType::ReqEvent(re) => Some(re),
			_ => None,
		}).expect("No last reqevent")
	}
	fn last_enum(&mut self) -> &mut Enum {
		self.elements.iter_mut().rev().find_map(|e| match e {
			ElementType::Enum(e) => Some(e),
			_ => None,
		}).expect("No last enum")
	}
	fn last_description(&mut self) -> &mut Description {
		self.elements.iter_mut().rev().find_map(|e| match e {
			ElementType::Description(d) => Some(d),
			_ => None,
		}).expect("No last description")
	}
	fn take_last_interface(&mut self) -> Interface {
		let index = self.elements
			.iter()
			.rposition(|e| matches!(e, ElementType::Interface(_)));

		match self.elements.remove(index.unwrap()) {
			ElementType::Interface(i) => Some(i),
			_ => None,
		}.expect("Couldn't take; no last interface")
	}
	fn take_last_reqevent(&mut self) -> RequestEvent {
		let index = self.elements
			.iter()
			.rposition(|e| matches!(e, ElementType::ReqEvent(_)));

		match self.elements.remove(index.unwrap()) {
			ElementType::ReqEvent(re) => Some(re),
			_ => None,
		}.expect("Couldn't take; no last reqevent")
	}
	fn take_last_enum(&mut self) -> Enum {
		let index = self.elements
			.iter()
			.rposition(|e| matches!(e, ElementType::Enum(_)));

		match self.elements.remove(index.unwrap()) {
			ElementType::Enum(e) => Some(e),
			_ => None,
		}.expect("Couldn't take; no last enum")
	}
	fn take_last_description(&mut self) -> Description {
		let index = self.elements
			.iter()
			.rposition(|e| matches!(e, ElementType::Description(_)));

		match self.elements.remove(index.unwrap()) {
			ElementType::Description(d) => Some(d),
			_ => None,
		}.expect("Couldn't take; no last description")
	}
	fn update_element_description(&mut self) {
		let desc = self.take_last_description();
		match self.last() {
			ElementType::Interface(i) => {
				i.description = Some(desc);
			},
			ElementType::ReqEvent(re) => {
				re.description = Some(desc);
			},
			ElementType::Enum(e) => {
				e.description = Some(desc);
			},
			_ => {},
		}
	}
}

fn build_output(interfaces: Vec<Interface>) -> String {
	let mut output = String::new();

	for i in interfaces {
		let name = format::snake_to_upper_camel(&i.name);
		if let Some(desc) = i.description.as_ref() {
			if let Some(content) = &desc.content {
				for line in content.split("\n") {
					output += format!("/// {}\n", line.trim()).as_str();
				}
			}
		}
		output += format!("pub mod {} {{\n", name).as_str();
		output += format!("\tpub const VERSION: u32 = {};\n\n", i.version).as_str();
		output += "\tpub enum Request {\n";
		for r in &i.requests {
			let name = format::snake_to_upper_camel(&r.name);
			if let Some(desc) = r.description.as_ref() {
				if let Some(content) = &desc.content {
					for line in content.split("\n") {
						output += format!("\t\t/// {}\n", line.trim()).as_str();
					}
				}
			}
			output += format!("\t\t {},\n", name).as_str();
		}
		output += "\t}\n\n";
		output += "\tpub enum Event {\n";
		for e in &i.events {
			if let Some(desc) = e.description.as_ref() {
				if let Some(content) = &desc.content {
					for line in content.split("\n") {
						output += format!("\t\t/// {}\n", line.trim()).as_str();
					}
				}
			}
			let name = format::snake_to_upper_camel(&e.name);
			output += "\t\t";
			output += name.as_str();
			output += " {},\n";
		}
		output += "\t}\n";

		if !i.enums.is_empty() {
			output += "\n";
		}

		for e in &i.enums {
			let name = format::snake_to_upper_camel(&e.name);
			if let Some(desc) = e.description.as_ref() {
				if let Some(content) = &desc.content {
					for line in content.split("\n") {
						output += format!("\t/// {}\n", line.trim()).as_str();
					}
				}
			}
			output += "\t#[repr(u32)]\n";
			output += format!("\tpub enum {} {{\n", name).as_str();
			for entry in &e.values {
				let name = format::snake_to_upper_camel(&entry.name);
				output += format!("\t\t{} = {},\n", name, entry.value).as_str();
			}
			output += "\t}\n\n";
			output += format!("\timpl {} {{\n", name).as_str();
			output += "\t\tpub fn from_u32(value: u32) -> Option<Self> {\n";
			output += "\t\t\tmatch value {\n";
			for entry in &e.values {
				let name = format::snake_to_upper_camel(&entry.name);
				output += format!("\t\t\t\t{} => Some(Format::{}),\n", entry.value, name).as_str();
			}
			output += "\t\t\t\t_ => None,\n";
			output += "\t\t\t}\n";
			output += "\t\t}\n";
			output += "\t}\n\n";
		}

		output += "}\n\n";
	}

	output
}

fn parse_xml(reader: &mut Reader<BufReader<File>>) -> anyhow::Result<Vec<Interface>> {
	let mut buf = Vec::new();

	let mut stack = Stack {
		elements: vec![],
		interfaces: vec![],
	};

	let mut element_stack = vec![];

	while let Ok(event) = reader.read_event_into(&mut buf) {
		match event {
			Start(ref e) => {
				let name = e.name();
				let name = name.as_ref();
				let element = std::str::from_utf8(name)?.to_owned();
				if !e.is_empty() {
					element_stack.push(element);
				}
				match name {
					b"interface" => {
						let attributes = xml::parse_attributes(e)?;
						let name = xml::find_attr(&attributes, "name");
						let version = xml::find_attr(&attributes, "version");

						stack.elements.push(ElementType::Interface(Interface {
							name,
							version: u32::from_str_radix(&version, 10)?,
							description: None,
							requests: vec![],
							events: vec![],
							enums: vec![],
						}));
					},
					b"request" => {
						let attributes = xml::parse_attributes(e)?;
						let name = xml::find_attr(&attributes, "name");

						stack.elements.push(ElementType::ReqEvent(RequestEvent {
							name,
							description: None,
							args: vec![],
						}));
					},
					b"event" => {
						let attributes = xml::parse_attributes(e)?;
						let name = xml::find_attr(&attributes, "name");

						stack.elements.push(ElementType::ReqEvent(RequestEvent {
							name,
							description: None,
							args: vec![],
						}));
					},
					b"enum" => {
						let attributes = xml::parse_attributes(e)?;
						let name = xml::find_attr(&attributes, "name");

						stack.elements.push(ElementType::Enum(Enum {
							name,
							description: None,
							values: vec![],
						}));
					},
					b"description" => {
						let attributes = xml::parse_attributes(e)?;
						let summary = xml::find_attr(&attributes, "summary");

						stack.elements.push(ElementType::Description(Description {
							summary,
							content: None,
						}));
					},
					_ => {},
				}
			}
			Empty(ref e) => {
				match e.name().as_ref() {
					b"arg" => {
						let attributes = xml::parse_attributes(e)?;
						let name = xml::find_attr(&attributes, "name");
						let typ = xml::find_attr(&attributes, "type");
						let summary = xml::find_attr(&attributes, "summary");
						let allow_null = xml::try_find_attr(&attributes, "allow-null");
						let allow_null = allow_null.as_deref();
						let allow_null = match allow_null.unwrap_or("false") {
							"true" => Some(true),
							"false" => Some(false),
							_ => None
						}.unwrap();

						let re = stack.last_reqevent();
						re.args.push(Arg {
							name,
							typ: convert::type_from_string(typ),
							summary,
							allow_null,
						});
					}
					b"entry" => {
						let attributes = xml::parse_attributes(e)?;
						let name = xml::find_attr(&attributes, "name");
						let name = format::sanitize_enum_entry_name(name);
						let summary = xml::try_find_attr(&attributes, "summary");
						let value = xml::find_attr(&attributes, "value");

						let e = stack.last_enum();
						e.values.push(EnumEntry {
							name,
							summary,
							value,
						});
					}
					b"description" => {
						let attributes = xml::parse_attributes(e)?;
						let summary = xml::find_attr(&attributes, "summary");

						stack.elements.push(ElementType::Description(Description {
							summary,
							content: None,
						}));
						stack.update_element_description();
					}
					_ => {}
				}
			}
			Text(ref e) => {
				let text = str::from_utf8(e.trim_ascii())?;
				let text = format::format_documentation(text)?;

				if let Some(current_element) = element_stack.last().map(|e| e.as_str()) {
					match current_element {
						"description" => {
							let desc = stack.last_description();
							desc.content = Some(text.to_string());
						},
						_ => {},
					}
				}
			}
			CData(_) => {}
			Comment(_) => {}
			Decl(_) => {}
			PI(_) => {}
			DocType(_) => {}
			End(ref e) => {
				element_stack.pop();
				match e.name().as_ref() {
					b"interface" => {
						let i = stack.take_last_interface();
						stack.interfaces.push(i);
					},
					b"request" => {
						let re = stack.take_last_reqevent();
						stack.last_interface().requests.push(re);
					},
					b"event" => {
						let re = stack.take_last_reqevent();
						stack.last_interface().events.push(re);
					},
					b"enum" => {
						let e = stack.take_last_enum();
						stack.last_interface().enums.push(e);
					}
					b"description" => {
						stack.update_element_description();
					},
					_ => {},
				}
			}
			Eof => {
				println!("Reached end of file.");
				break;
			}
		};
	}

	return Ok(stack.interfaces);
}

fn main() -> anyhow::Result<()> {
	const INPUT_FILE: &str = "/usr/share/wayland/wayland.xml";
	const OUTPUT_FILE: &str = "output.rs";

	let mut reader = Reader::from_file(INPUT_FILE)?;
	let mut writer = File::create(OUTPUT_FILE)?;

	let interfaces = parse_xml(&mut reader)?;

	println!("Building output...");
	let output = build_output(interfaces);

	println!("Saving output...");
	writer.write_all(output.as_bytes())?;
	println!("Output generated successfully.");

	Ok(())
}
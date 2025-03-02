use core::str;
use std::collections::HashSet;
use std::{fs::File, io::Write};
use std::io::BufReader;
use format::snake_to_upper_camel;
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
mod format;
mod xml;

use wl_types::{
	ArgType,
	Interface,
	Description,
	RequestEvent,
	Arg,
	EnumEntry,
	Enum,
};

const INPUT_FILE: &str = "/usr/share/wayland/wayland.xml";
const OUTPUT_FILE: &str = "output/src/lib.rs";
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

fn t(count: usize) -> String {
	let mut tabs = String::new();
	for _ in 0..count {
		tabs += "\t";
	}
	tabs
}

fn output_string_conversion() -> String {
	let mut output = String::new();

	output += "fn le_arr_to_string(words: &[u32], length: usize) -> String {\n";
	output += "\tlet mut bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();\n";
	output += "\tbytes.truncate(length);\n";
	output += "\tString::from_utf8_lossy(&bytes).to_string()\n";
	output += "}\n\n";

	output
}

fn string_enum_to_type(enm: &String) -> String {
	let parts: Vec<&str> = enm.split('.').collect();
	match parts.len() {
		1 => {
			Some(format::snake_to_upper_camel(parts[0]))
		}
		2 => {
			let module = parts[0];
			let name = format::snake_to_upper_camel(parts[1]);
			Some(format!("{}::{}", module, name))
		}
		_ => None
	}.expect("Enum had three or more parts")
}

fn output_reqevent(re: &RequestEvent) -> String {
	let mut output = String::new();

	if let Some(desc) = re.description.as_ref() {
		if let Some(content) = &desc.content {
			for line in content.split("\n") {
				output += format!("\t\t/// {}\n", line.trim()).as_str();
			}
		}
	}
	let name = format::snake_to_upper_camel(&re.name);
	if re.args.len() == 0 {
		output += format!("\t\t{} {{}},\n", name).as_str();
	} else {
		output += format!("\t\t{} {{\n", name).as_str();
		for a in &re.args {
			output += format!("\t\t\t/// {}.\n", format::to_title(&a.summary)).as_str();
			if let Some(enm) = &a.enm {
				output += format!("\t\t\t{}: {},\n", a.name, string_enum_to_type(enm)).as_str();
			} else {
				output += format!("\t\t\t{}: {},\n", a.name, a.typ.to_rust_type_string()).as_str();
			}
		}
		output += "\t\t},\n"
	}

	output
}

fn output_request_from_opcode(re: &RequestEvent, index: usize) -> String {
	let mut output = String::new();

	output += format!("{}{} => {{\n", t(4), index).as_str();
	if re.args.len() == 1 {
		output += format!("{}let index = 0usize;\n", t(5)).as_str();
	} else if re.args.len() >= 1 {
		output += format!("{}let mut index = 0usize;\n", t(5)).as_str();
	}
	for (index, arg) in re.args.iter().enumerate() {
		let last_arg = index == re.args.len() - 1;
		let single_byte = match arg.typ {
			ArgType::Int => true,
			ArgType::Uint => true,
			ArgType::Fixed => true,
			ArgType::String => false,
			ArgType::ObjectId => true,
			ArgType::NewId => true,
			ArgType::Array => false,
			ArgType::Fd => true,
		};
		if single_byte {
			match arg.typ {
				ArgType::Int => {
					output += format!(
						"{}let {} = args[index] as i32;\n",
						t(5), arg.name,
					).as_str();
				}
				ArgType::Uint => {
					if let Some(enm) = &arg.enm {
						let typ = string_enum_to_type(enm);
						output += format!(
							"{}let {} = {}::from_u32(args[index]).unwrap();\n",
							t(5), arg.name, typ,
						).as_str();
					} else {
						output += format!(
							"{}let {} = args[index];\n",
							t(5), arg.name,
						).as_str();
					}
				}
				_ => {
					output += format!(
						"{}let {} = args[index];\n",
						t(5), arg.name,
					).as_str();
				}
			}
			if !last_arg {
				output += format!("{}index += 1;\n", t(5)).as_str();
			}
		} else {
			output += match arg.typ {
				ArgType::String => {
					let mut output = String::new();
					output += format!(
						"{}let {}_size: usize = args[index].try_into().unwrap();\n",
						t(5), arg.name,
					).as_str();
					// The word count (padded), not the number of characters
					output += format!(
						"{}let {}_count = {}_size.div_ceil(4);\n",
						t(5), arg.name, arg.name,
					).as_str();
					output += format!(
						"{}let {} = crate::le_arr_to_string(&args[index + 1..index + 1 + {}_count], {}_size);\n",
						t(5), arg.name, arg.name, arg.name,
					).as_str();
					if !last_arg {
						output += format!(
							"{}index += 1 + {}_count;\n",
							t(5), arg.name,
						).as_str();
					}
					Some(output)
				}
				ArgType::Array => {
					let mut output = String::new();
					output += format!(
						"{}let {}_size: usize = args[index].try_into().unwrap();\n",
						t(5), arg.name,
					).as_str();
					// The word count (padded), not the number of elements
					output += format!(
						"{}let {}_count = {}_size.div_ceil(4);\n",
						t(5), arg.name, arg.name,
					).as_str();
					output += format!(
						"{}let {} = crate::to_vec(&args[index + 1..index + 1 + {}_count], {}_size);\n",
						t(5), arg.name, arg.name, arg.name,
					).as_str();
					if !last_arg {
						output += format!(
							"{}index += 1 + {}_count;\n",
							t(5), arg.name,
						).as_str();
					}
					Some(output)
				}
				_ => None
			}.expect("Argument type was multi-byte but no handler was provided").as_str();
		}
	}

	output += format!(
		"{}Some(Request::{} {{\n",
		t(5), snake_to_upper_camel(&re.name),
	).as_str();
	for arg in &re.args {
		output += format!("{}{},\n", t(6), arg.name).as_str();
	}
	output += format!("{}}})\n", t(5)).as_str();
	output += format!("{}}}\n", t(4)).as_str();

	output
}

fn build_output(interfaces: Vec<Interface>) -> String {
	let mut output = String::new();
	output += output_string_conversion().as_str();

	for (index, interface) in interfaces.iter().enumerate() {
		if let Some(desc) = interface.description.as_ref() {
			if let Some(content) = &desc.content {
				for line in content.split("\n") {
					output += format!("/// {}\n", line.trim()).as_str();
				}
			}
		}
		output += format!("pub mod {} {{\n", &interface.name).as_str();

		// Include modules for request arguments that reference enums outside of
		// their own module
		let chain: Vec<&RequestEvent> = interface.requests.iter().chain(interface.events.iter()).collect();
		let mut modules = HashSet::new();
		for re in chain {
			for a in &re.args {
				if let Some(enm) = &a.enm {
					let parts: Vec<&str> = enm.split('.').collect();
					match parts.len() {
						2 => {
							modules.insert(parts[0]);
						}
						_ => {}
					}
				}
			}
		}
		if !modules.is_empty() {
			for entry in modules {
				output += format!("\tuse crate::{};\n", entry).as_str();
			}
			output += "\n";
		}

		output += format!("\tpub const VERSION: u32 = {};\n", interface.version).as_str();

		if !interface.requests.is_empty() {
			output += "\n\tpub enum Request {\n";
			for request in &interface.requests {
				output += output_reqevent(request).as_str();
			}
			output += "\t}\n\n";

			output += "\timpl Request {\n";
			output += "\t\tpub fn from_opcode(opcode: u16, args: &[u32]) -> Option<Self> {\n";
			output += format!("{}match opcode {{\n", t(3)).as_str();
			for (i, event) in interface.requests.iter().enumerate() {
				output += output_request_from_opcode(event, i).as_str();
			}
			output += format!("{}_ => None,\n", t(4)).as_str();
			output += format!("{}}}\n", t(3)).as_str();
			output += "\t\t}\n";
			output += "\t}\n"
		}

		if !interface.events.is_empty() {
			output += "\n\tpub enum Event {\n";
			for event in &interface.events {
				output += output_reqevent(event).as_str();
			}
			output += "\t}\n";
		}

		if !interface.enums.is_empty() {
			output += "\n";
		}
		for (index, enm) in interface.enums.iter().enumerate() {
			let name = format::snake_to_upper_camel(&enm.name);
			if let Some(desc) = enm.description.as_ref() {
				if let Some(content) = &desc.content {
					for line in content.split("\n") {
						output += format!("\t/// {}\n", line.trim()).as_str();
					}
				}
			}
			output += "\t#[repr(u32)]\n";
			output += format!("\tpub enum {} {{\n", name).as_str();
			for entry in &enm.values {
				let name = format::snake_to_upper_camel(&entry.name);
				output += format!("\t\t{} = {},\n", name, entry.value).as_str();
			}
			output += "\t}\n\n";
			output += format!("\timpl {} {{\n", name).as_str();
			output += "\t\tpub fn from_u32(value: u32) -> Option<Self> {\n";
			output += "\t\t\tmatch value {\n";
			for entry in &enm.values {
				let entry_name = format::snake_to_upper_camel(&entry.name);
				output += format!("\t\t\t\t{} => Some({}::{}),\n", entry.value, name, entry_name).as_str();
			}
			output += "\t\t\t\t_ => None,\n";
			output += "\t\t\t}\n";
			output += "\t\t}\n";
			output += "\t}\n";
			if index < interface.enums.len() - 1 {
				output += "\n";
			}
		}

		output += "}\n";
		if index < interfaces.len() - 1 {
			output += "\n";
		}
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
						let enm = xml::try_find_attr(&attributes, "enum");

						let re = stack.last_reqevent();
						re.args.push(Arg {
							name,
							typ: ArgType::from_string(typ),
							summary,
							enm,
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
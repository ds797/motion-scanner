use std::fmt::Display;

/// Wire format
pub enum ArgType {
	/// 32-bit signed integer
	Int,				
	/// 32-bit unsigned integer
	Uint,
	/// Signed 24.8 decimal number
	Fixed,
	/// 32-bit length (incl. null terminator), followed by string contents, then
	/// null terminated, padded to 32-bit boundary. A null value is represented
	/// with a value of 0.
	String,
	/// 32-bit object ID. A null value is represented with an ID of 0.
	ObjectId,
	/// The 32-bit object ID. Generally, the interface used for the new object is
	/// inferred from the XML, but in the case where it's not specified, a new_id
	/// is preceded by a `string` specifying the interface name, and a `uint`
	/// specifying the version.
	NewId,
	/// Starts with 32-bit array size in bytes, followed by the array contents
	/// verbatim, and finally padding to a 32-bit boundary.
	Array,
	/// The file descriptor is not stored in the message buffer, but in the
	/// ancillary data of the UNIX domain socket message (msg_control).
	Fd,
}

impl ArgType {
	pub fn from_string(typ: String) -> ArgType {
		match typ.as_str() {
			"int" => Some(ArgType::Int),
			"uint" => Some(ArgType::Uint),
			"fixed" => Some(ArgType::Fixed),
			"string" => Some(ArgType::String),
			"object" => Some(ArgType::ObjectId),
			"new_id" => Some(ArgType::NewId),
			"array" => Some(ArgType::Array),
			"fd" => Some(ArgType::Fd),
			_ => None
		}.expect("Unknown type")
	}
	
	pub fn to_rust_type_string(&self) -> String {
		match self {
			ArgType::Int => "i32".to_string(),
			ArgType::Uint => "u32".to_string(),
			ArgType::Fixed => "f32".to_string(),
			ArgType::String => "String".to_string(),
			ArgType::ObjectId => "u32".to_string(),
			ArgType::NewId => "u32".to_string(),
			// This is hardcoded for now - there's only one instance of an
			// array type in the entire protocol. However, this might change -
			// ideally we'd have a more robust type system for arrays.
			ArgType::Array => "Vec<u32>".to_string(),
			ArgType::Fd => "u32".to_string(),
		}
	}
}

impl ToString for ArgType {
	fn to_string(&self) -> String {
		match self {
			ArgType::Int => "int".to_string(),
			ArgType::Uint => "uint".to_string(),
			ArgType::Fixed => "float".to_string(),
			ArgType::String => "string".to_string(),
			ArgType::ObjectId => "object_id".to_string(),
			ArgType::NewId => "new_id".to_string(),
			ArgType::Array => "array".to_string(),
			ArgType::Fd => "fd".to_string(),
		}
	}
}

pub struct Interface {
	pub name: String,
	pub version: u32,
	pub description: Option<Description>,
	pub requests: Vec<RequestEvent>,
	pub events: Vec<RequestEvent>,
	pub enums: Vec<Enum>,
}

impl Display for Interface {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Interface {}, version {}\n", self.name, self.version)?;
		write!(f, " - Requests:\n")?;
		for r in &self.requests {
			write!(f, "  - {}: {}, with arguments:\n", r.name, r.description.as_ref().expect("No request summary").summary)?;
			for a in &r.args {
				write!(f, "   - {}\n", a)?;
			}
		}
		write!(f, "\n - Events:\n")?;
		for e in &self.events {
			write!(f, "  - {}: {}\n", e.name, e.description.as_ref().expect("No event summary").summary)?;
			for a in &e.args {
				write!(f, "   - {}\n", a)?;
			}
		}
		write!(f, "\n")?;
		Ok(())
	}
}

pub struct Description {
	pub summary: String,
	pub content: Option<String>,
}

pub struct RequestEvent {
	pub name: String,
	pub description: Option<Description>,
	pub args: Vec<Arg>,
}

pub struct Arg {
	pub name: String,
	pub typ: ArgType,
	pub summary: String,
	pub enm: Option<String>,
	pub allow_null: bool,
}

impl Display for Arg {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let s = match self.allow_null {
			true => "?",
			false => "",
		};
		write!(f, "{}{} {}: {}", self.typ.to_string(), s, self.name, self.summary)?;
		Ok(())
	}
}

pub struct Enum {
	pub name: String,
	pub description: Option<Description>,
	pub values: Vec<EnumEntry>,
}

impl std::fmt::Display for Enum {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Enum {} - {}", self.name, self.description.as_ref().expect("No enum description").summary)?;
		Ok(())
	}
}

pub struct EnumEntry {
	pub name: String,
	pub summary: Option<String>,
	pub value: String,
}
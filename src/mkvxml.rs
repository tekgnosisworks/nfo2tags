use std::fs::File;
use std::io::Write;
use xml::reader::{EventReader, XmlEvent};
use xml::writer::{EmitterConfig, EventWriter};

pub fn convert_to_mkv_tags(input_xml_path: &str, output_xml_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(input_xml_path)?;
    let parser = EventReader::new(file);

    let output_file = File::create(output_xml_path)?;
    let config = EmitterConfig::new().perform_indent(true); // For pretty printing (optional)
    let mut writer = EventWriter::new_with_config(output_file, config);

    writer.write(xml::writer::XmlEvent::StartDocument {
        version: xml::common::XmlVersion::Version10,
        encoding: Some("UTF-8"),
        standalone: None,
    })?;

    writer.write(xml::writer::XmlEvent::start_element("Tags"))?;
    writer.write(xml::writer::XmlEvent::start_element("Tag"))?;

    let mut current_tag_name: Option<String> = None;
    let mut directors = Vec::new();
    let mut writers = Vec::new();
    let mut actors = Vec::new();
    let mut collection_name = None;
    let mut collection_overview = None;
    let mut plot = None;
    let mut outline = None;

    let mut inside_actor = false;
    let mut inside_name = false;
    let mut current_actor_name = String::new();

    for e in parser {
        match e? {
            XmlEvent::StartElement { name, .. } => {
                current_tag_name = Some(name.local_name.clone());
                match name.local_name.as_str() {
                    "actor" => inside_actor = true,
                    "name" if inside_actor => inside_name = true,
                    _ => {}
                }
            }
            XmlEvent::Characters(text) => {
                match current_tag_name.as_ref().map(|s| s.as_str()) {
                    Some("name") if inside_actor && inside_name => {
                        current_actor_name = text.to_string();
                    }
                    Some("director") => {
                        directors.push(text.to_string());
                    }
                    Some("credits") => {
                        writers.push(text.to_string());
                    }
                    Some("plot") => plot = Some(text.to_string()),
                    Some("outline") => outline = Some(text.to_string()),
                    Some("overview") => collection_overview = Some(text.to_string()),
                    Some("name") if current_tag_name.as_ref().map(|s| s.as_str()) == Some("set") => {
                        collection_name = Some(text.to_string())
                    }
                    Some("genre") => write_simple_tag(&mut writer, "GENRE", &text)?,
                    Some("id") => write_simple_tag(&mut writer, "IMDB", &text)?,
                    Some("title") => write_simple_tag(&mut writer, "TITLE", &text)?,
                    Some("originaltitle") => write_simple_tag(&mut writer, "ORIGINALTITLE", &text)?,
                    Some("year") => write_simple_tag(&mut writer, "YEAR", &text)?,
                    Some("tagline") => write_simple_tag(&mut writer, "TAGLINE", &text)?,
                    Some("runtime") => write_simple_tag(&mut writer, "RUNTIME", &text)?,
                    Some("mpaa") => write_simple_tag(&mut writer, "MPAA", &text)?,
                    Some("certification") => write_simple_tag(&mut writer, "CERTIFICATION", &text)?,
                    Some("tmdbid") => write_simple_tag(&mut writer, "TMDB", &text)?, // Changed to TMDB
                    Some("country") => write_simple_tag(&mut writer, "COUNTRY", &text)?,
                    Some("premiered") => write_simple_tag(&mut writer, "PREMIERED", &text)?,
                    Some("studio") => write_simple_tag(&mut writer, "STUDIO", &text)?,
                    _ => {}
                }
            }
            XmlEvent::EndElement { name } => {
                match name.local_name.as_str() {
                    "actor" => {
                        if !current_actor_name.is_empty() {
                            actors.push(current_actor_name.clone());
                            current_actor_name.clear();
                        }
                        inside_actor = false;
                    }
                    "name" if inside_actor => inside_name = false,
                    "movie" => {
                        write_collected_tags(&mut writer, &[
                            ("DESCRIPTION", plot.as_ref()),
                            ("SUMMARY", outline.as_ref()),
                            ("Collection Name", collection_name.as_ref()),
                            ("Collection Overview", collection_overview.as_ref()),
                        ])?;

                        write_list_tags(&mut writer, &[
                            ("Director", &directors),
                            ("WRITER", &writers),
                            ("Actor", &actors),
                        ])?;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    writer.write(xml::writer::XmlEvent::end_element())?; // </Tag>
    writer.write(xml::writer::XmlEvent::end_element())?; // </Tags>

    Ok(())
}

fn write_simple_tag<W: Write>(writer: &mut EventWriter<W>, name: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    writer.write(xml::writer::XmlEvent::start_element("Simple"))?;
    
    writer.write(xml::writer::XmlEvent::start_element("Name"))?;
    writer.write(xml::writer::XmlEvent::characters(name))?;
    writer.write(xml::writer::XmlEvent::end_element())?;
    
    writer.write(xml::writer::XmlEvent::start_element("String"))?;
    writer.write(xml::writer::XmlEvent::characters(value))?;
    writer.write(xml::writer::XmlEvent::end_element())?;
    
    writer.write(xml::writer::XmlEvent::end_element())?;
    Ok(())
}

fn write_collected_tags<W: Write>(
    writer: &mut EventWriter<W>, 
    tags: &[(&str, Option<&String>)]
) -> Result<(), Box<dyn std::error::Error>> {
    for (name, value) in tags {
        if let Some(v) = value {
            write_simple_tag(writer, name, v)?;
        }
    }
    Ok(())
}

fn write_list_tags<W: Write>(
    writer: &mut EventWriter<W>,
    tags: &[(&str, &Vec<String>)]
) -> Result<(), Box<dyn std::error::Error>> {
    for (name, values) in tags {
        if !values.is_empty() {
            write_simple_tag(writer, name, &values.join(","))?;
        }
    }
    Ok(())
}



pub mod auth;
pub mod client;
#[cfg(test)]
mod tests;

use auth::username_token::UsernameToken;
use schema::soap_envelope;
use xmltree::{Element, Namespace, XMLNode};

const SOAP_URI: &str = "http://www.w3.org/2003/05/soap-envelope";

#[derive(Debug)]
pub enum Error {
    ParseError(xmltree::ParseError),
    EnvelopeNotFound,
    BodyNotFound,
    BodyIsEmpty,
    Fault(Box<soap_envelope::Fault>),
    InternalError(String),
}

#[derive(Debug)]
pub struct Response {
    pub response: Option<String>,
}

pub fn soap(
    xml: &str,
    username_token: &Option<UsernameToken>,
    address_to: Option<String>,
    action: Option<String>,
) -> Result<String, Error> {
    let soap_prefix = "s";
    let app_data = parse(xml)?;

    let mut namespaces = app_data.namespaces.clone().unwrap_or_else(Namespace::empty);
    namespaces.put(soap_prefix, SOAP_URI);
    if address_to.is_some() || action.is_some() {
        namespaces.put("wsa", "http://www.w3.org/2005/08/addressing");
    }

    let mut body = Element::new("Body");
    body.prefix = Some(soap_prefix.to_string());
    body.children.push(XMLNode::Element(app_data));

    let mut envelope = Element::new("Envelope");
    envelope.namespaces = Some(namespaces);
    envelope.prefix = Some(soap_prefix.to_string());

    let mut header_elements: Vec<Element> = Vec::new();
    if let Some(username_token) = username_token {
        header_elements.push(parse(&username_token.to_xml())?);
    }
    if let Some(address_to) = address_to {
        let mut element_to = parse(&format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
                <wsa:To xmlns:wsa="http://www.w3.org/2005/08/addressing">{address_to}</wsa:To>"##
        ))?;
        element_to
            .attributes
            .insert(format!("{soap_prefix}:mustUnderstand"), "true".to_string());
        header_elements.push(element_to);
    }
    if let Some(action) = action {
        let mut element_to = parse(&format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
                <wsa:Action xmlns:wsa="http://www.w3.org/2005/08/addressing">{action}</wsa:Action>"##
        ))?;
        element_to
            .attributes
            .insert(format!("{soap_prefix}:mustUnderstand"), "true".to_string());
        header_elements.push(element_to);
    }

    if !header_elements.is_empty() {
        let mut header = Element::new("Header");
        header.prefix = Some(soap_prefix.to_string());

        header_elements
            .iter()
            .for_each(|element| header.children.push(XMLNode::Element(element.clone())));

        envelope.children.push(XMLNode::Element(header));
    }

    envelope.children.push(XMLNode::Element(body));

    xml_element_to_string(&envelope)
}

pub fn unsoap(xml: &str) -> Result<String, Error> {
    let root = parse(xml)?;

    if root.name != "Envelope" {
        return Err(Error::EnvelopeNotFound);
    }

    let body = root.get_child("Body").ok_or(Error::BodyNotFound)?;

    if let Some(fault) = body.get_child("Fault") {
        let fault = deserialize_fault(fault)?;
        return Err(Error::Fault(Box::new(fault)));
    }

    body.children
        .iter()
        .find_map(|node| match node {
            XMLNode::Element(app_data) => Some(xml_element_to_string(app_data)),
            _ => None,
        })
        .ok_or(Error::BodyIsEmpty)?
}

fn parse(xml: &str) -> Result<Element, Error> {
    Element::parse(xml.as_bytes()).map_err(Error::ParseError)
}

fn xml_element_to_string(el: &Element) -> Result<String, Error> {
    let mut out = vec![];
    el.write(&mut out)
        .map_err(|_| Error::InternalError("Could not write XML element".to_string()))?;
    String::from_utf8(out).map_err(|e| Error::InternalError(e.to_string()))
}

fn deserialize_fault(envelope: &Element) -> Result<soap_envelope::Fault, Error> {
    let string = xml_element_to_string(envelope)?;
    yaserde::de::from_str(&string).map_err(Error::InternalError)
}

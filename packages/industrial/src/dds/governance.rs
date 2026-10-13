use super::ffi::{Ptr, error};
use crate::{Result, require};
use libloading::Library;
use std::{ffi::c_void, ptr};

// Use the same S/MIME verification API and flags as Cyclone DDS access control.
// The XML we inspect is the verified content, never an independently parsed MIME part.
#[allow(non_snake_case)]
struct Crypto {
    BIO_new_mem_buf: unsafe extern "C" fn(*const c_void, i32) -> Ptr,
    BIO_new: unsafe extern "C" fn(Ptr) -> Ptr,
    BIO_s_mem: unsafe extern "C" fn() -> Ptr,
    BIO_free: unsafe extern "C" fn(Ptr) -> i32,
    BIO_read: unsafe extern "C" fn(Ptr, Ptr, i32) -> i32,
    PEM_read_bio_X509: unsafe extern "C" fn(Ptr, Ptr, Ptr, Ptr) -> Ptr,
    X509_free: unsafe extern "C" fn(Ptr),
    SMIME_read_PKCS7: unsafe extern "C" fn(Ptr, *mut Ptr) -> Ptr,
    PKCS7_verify: unsafe extern "C" fn(Ptr, Ptr, Ptr, Ptr, Ptr, i32) -> i32,
    PKCS7_free: unsafe extern "C" fn(Ptr),
    OPENSSL_sk_new_null: unsafe extern "C" fn() -> Ptr,
    OPENSSL_sk_push: unsafe extern "C" fn(Ptr, Ptr) -> i32,
    OPENSSL_sk_free: unsafe extern "C" fn(Ptr),
    _library: Library,
}
impl Crypto {
    fn load() -> Result<Self> {
        #[cfg(target_os = "windows")]
        const NAMES: &[&str] = &[
            "libcrypto-3-x64.dll",
            "libcrypto-3-arm64.dll",
            "libcrypto-3.dll",
        ];
        #[cfg(target_os = "macos")]
        const NAMES: &[&str] = &["libcrypto.3.dylib"];
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        const NAMES: &[&str] = &["libcrypto.so.3"];
        let library = NAMES
            .iter()
            .find_map(|name| unsafe { Library::new(name).ok() })
            .ok_or_else(|| {
                error("OpenSSL 3 libcrypto runtime is not installed or cannot be loaded")
            })?;
        macro_rules! symbol {
            ($name:ident) => {
                unsafe {
                    *library
                        .get(concat!(stringify!($name), "\0").as_bytes())
                        .map_err(|_| error("OpenSSL 3 runtime is missing a required API"))?
                }
            };
        }
        Ok(Self {
            BIO_new_mem_buf: symbol!(BIO_new_mem_buf),
            BIO_new: symbol!(BIO_new),
            BIO_s_mem: symbol!(BIO_s_mem),
            BIO_free: symbol!(BIO_free),
            BIO_read: symbol!(BIO_read),
            PEM_read_bio_X509: symbol!(PEM_read_bio_X509),
            X509_free: symbol!(X509_free),
            SMIME_read_PKCS7: symbol!(SMIME_read_PKCS7),
            PKCS7_verify: symbol!(PKCS7_verify),
            PKCS7_free: symbol!(PKCS7_free),
            OPENSSL_sk_new_null: symbol!(OPENSSL_sk_new_null),
            OPENSSL_sk_push: symbol!(OPENSSL_sk_push),
            OPENSSL_sk_free: symbol!(OPENSSL_sk_free),
            _library: library,
        })
    }
    fn verified_xml(&self, document: &str, ca: &str) -> Result<String> {
        // Every object is freed by the same loaded library that allocated it.
        let mut objects = Objects {
            crypto: self,
            bios: Vec::new(),
            certificate: ptr::null_mut(),
            signed: ptr::null_mut(),
            stack: ptr::null_mut(),
        };
        unsafe {
            let input = (self.BIO_new_mem_buf)(document.as_ptr().cast(), document.len() as i32);
            objects.bios.push(input);
            let cert_input = (self.BIO_new_mem_buf)(ca.as_ptr().cast(), ca.len() as i32);
            objects.bios.push(cert_input);
            let output = (self.BIO_new)((self.BIO_s_mem)());
            objects.bios.push(output);
            require(
                objects.bios.iter().all(|p| !p.is_null()),
                "DDS Security document allocation failed",
            )?;
            objects.certificate = (self.PEM_read_bio_X509)(
                cert_input,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            require(
                !objects.certificate.is_null(),
                "DDS permissions CA must be a PEM certificate",
            )?;
            let mut content = ptr::null_mut();
            objects.signed = (self.SMIME_read_PKCS7)(input, &mut content);
            objects.bios.push(content);
            require(
                !objects.signed.is_null() && !content.is_null(),
                "DDS policy must be a detached S/MIME signed document",
            )?;
            objects.stack = (self.OPENSSL_sk_new_null)();
            require(
                !objects.stack.is_null(),
                "DDS Security certificate allocation failed",
            )?;
            require(
                (self.OPENSSL_sk_push)(objects.stack, objects.certificate) > 0,
                "DDS Security certificate allocation failed",
            )?;
            require(
                (self.PKCS7_verify)(
                    objects.signed,
                    objects.stack,
                    ptr::null_mut(),
                    content,
                    output,
                    0x31,
                ) == 1,
                "DDS signed policy verification failed against the supplied permissions CA",
            )?;
            let mut bytes = vec![0u8; 1024 * 1024 + 1];
            let count = (self.BIO_read)(output, bytes.as_mut_ptr().cast(), bytes.len() as i32);
            require(
                count > 0 && count <= 1024 * 1024,
                "DDS verified policy exceeds its size limit or is empty",
            )?;
            bytes.truncate(count as usize);
            String::from_utf8(bytes).map_err(|_| error("DDS signed policy is not UTF-8 XML"))
        }
    }
}
struct Objects<'a> {
    crypto: &'a Crypto,
    bios: Vec<Ptr>,
    certificate: Ptr,
    signed: Ptr,
    stack: Ptr,
}
impl Drop for Objects<'_> {
    fn drop(&mut self) {
        unsafe {
            for bio in &self.bios {
                if !bio.is_null() {
                    (self.crypto.BIO_free)(*bio);
                }
            }
            if !self.signed.is_null() {
                (self.crypto.PKCS7_free)(self.signed);
            }
            if !self.stack.is_null() {
                (self.crypto.OPENSSL_sk_free)(self.stack);
            }
            if !self.certificate.is_null() {
                (self.crypto.X509_free)(self.certificate);
            }
        }
    }
}

pub(super) fn verify(config: &super::super::DdsSecurity) -> Result<()> {
    let crypto = Crypto::load()?;
    let xml = crypto.verified_xml(&config.governance_signed, &config.permissions_ca_pem)?;
    encrypted_governance(&xml)?;
    crypto.verified_xml(&config.permissions_signed, &config.permissions_ca_pem)?;
    Ok(())
}
fn child_value<'a>(node: roxmltree::Node<'a, 'a>, name: &str) -> Result<&'a str> {
    let mut children = node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == name);
    let child = children
        .next()
        .ok_or_else(|| error("governance omits a required protection policy"))?;
    require(
        children.next().is_none(),
        "DDS governance must not repeat protection policy elements",
    )?;
    child
        .text()
        .map(str::trim)
        .ok_or_else(|| error("governance protection policy is empty"))
}
fn encrypted(value: &str) -> bool {
    matches!(value, "ENCRYPT" | "ENCRYPT_WITH_ORIGIN_AUTHENTICATION")
}
fn encrypted_governance(xml: &str) -> Result<()> {
    let document = roxmltree::Document::parse(xml)
        .map_err(|_| error("verified governance is not valid XML"))?;
    // Cyclone accepts case-insensitive tag names. Reject aliases and namespaces here so
    // no second policy rule can bypass the checks through a different XML spelling.
    for node in document.descendants().filter(|n| n.is_element()) {
        require(
            node.tag_name().namespace().is_none()
                && node
                    .tag_name()
                    .name()
                    .bytes()
                    .all(|byte| !byte.is_ascii_uppercase()),
            "DDS governance element names must be lowercase and have no namespace",
        )?;
    }
    let mut domains = 0;
    for domain in document
        .descendants()
        .filter(|n| n.has_tag_name("domain_rule"))
    {
        domains += 1;
        require(
            child_value(domain, "allow_unauthenticated_participants")? == "false"
                && child_value(domain, "enable_join_access_control")? == "true",
            "DDS governance must reject unauthenticated participants and require join access control",
        )?;
        for name in [
            "discovery_protection_kind",
            "liveliness_protection_kind",
            "rtps_protection_kind",
        ] {
            require(
                encrypted(child_value(domain, name)?),
                "DDS governance must encrypt discovery, liveliness, and RTPS messages",
            )?;
        }
        let mut topics = 0;
        for topic in domain
            .descendants()
            .filter(|n| n.has_tag_name("topic_rule"))
        {
            topics += 1;
            for name in [
                "enable_discovery_protection",
                "enable_liveliness_protection",
                "enable_read_access_control",
                "enable_write_access_control",
            ] {
                require(
                    child_value(topic, name)? == "true",
                    "DDS governance must protect discovery, liveliness, and read/write topic access",
                )?;
            }
            require(
                encrypted(child_value(topic, "metadata_protection_kind")?)
                    && child_value(topic, "data_protection_kind")? == "ENCRYPT",
                "DDS governance must encrypt every topic's metadata and data",
            )?;
        }
        require(
            topics > 0,
            "DDS governance requires at least one protected topic rule",
        )?;
    }
    require(
        domains > 0,
        "DDS governance requires at least one protected domain rule",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn governance_rejects_missing_or_plaintext_protections() {
        let xml = "<dds><domain_access_rules><domain_rule><allow_unauthenticated_participants>false</allow_unauthenticated_participants><enable_join_access_control>true</enable_join_access_control><discovery_protection_kind>ENCRYPT</discovery_protection_kind><liveliness_protection_kind>ENCRYPT</liveliness_protection_kind><rtps_protection_kind>ENCRYPT</rtps_protection_kind><topic_access_rules><topic_rule><topic_expression>*</topic_expression><enable_discovery_protection>true</enable_discovery_protection><enable_liveliness_protection>true</enable_liveliness_protection><enable_read_access_control>true</enable_read_access_control><enable_write_access_control>true</enable_write_access_control><metadata_protection_kind>ENCRYPT</metadata_protection_kind><data_protection_kind>ENCRYPT</data_protection_kind></topic_rule></topic_access_rules></domain_rule></domain_access_rules></dds>";
        assert!(encrypted_governance(xml).is_ok());
        for insecure in [xml.replace("ENCRYPT", "NONE"), xml.replace("ENCRYPT", "SIGN"), xml.replace(">false<", ">true<"), xml.replace("<data_protection_kind>ENCRYPT</data_protection_kind>", ""), xml.replace("<data_protection_kind>ENCRYPT</data_protection_kind>", "<data_protection_kind>ENCRYPT</data_protection_kind><data_protection_kind>NONE</data_protection_kind>")] {
            assert!(encrypted_governance(&insecure).is_err());
        }
    }
}

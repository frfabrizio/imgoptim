// src/formats/xmp.rs
pub fn build_category_xmp(cat: &str) -> Vec<u8> {
    // XMP minimal (suffisant pour tests + compat Adobe)
    let s = format!(
        r#"<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description xmlns:xmp="http://ns.adobe.com/xap/1.0/"
                   xmlns:dc="http://purl.org/dc/elements/1.1/">
   <dc:subject>
    <rdf:Bag>
     <rdf:li>{}</rdf:li>
    </rdf:Bag>
   </dc:subject>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#,
        cat
    );
    s.into_bytes()
}

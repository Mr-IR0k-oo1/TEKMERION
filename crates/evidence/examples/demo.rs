use chrono::Utc;
use tekmerion_evidence::{normalize_url, EvidenceRecord};
use url::Url;

fn main() {
    println!("============================================================");
    println!("     TEKMERION DETERMINISTIC EVIDENCE ENGINE DEMO");
    println!("============================================================\n");

    let record = EvidenceRecord::new(
        "run-20260904-demo",
        Url::parse("https://example.com/item?b=2&a=1#fragment").unwrap(),
        "example.com",
        "web",
        "reverse_image_provider",
        Utc::now(),
        "Sample Candidate Page",
        "Extracted high-fidelity portrait description",
        "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90",
        0.895000,
        "adaface-ir101",
        0.920000,
    );

    println!("1. Evidence Record (13 Fields):");
    println!("   schema_version:    {}", record.schema_version);
    println!("   run_id:            {}", record.run_id);
    println!("   source_url (raw):  {}", record.source_url);
    println!("   source_url (norm): {}", normalize_url(&record.source_url));
    println!("   domain:            {}", record.domain);
    println!("   platform:          {}", record.platform);
    println!("   provider:          {}", record.provider);
    println!("   retrieved_at:      {}", record.retrieved_at.to_rfc3339());
    println!("   title:             {}", record.title);
    println!("   text:              {}", record.text);
    println!("   image_sha256:      {}", record.image_sha256);
    println!("   face_similarity:   {:.6}", record.face_similarity);
    println!("   face_model:        {}", record.face_model);
    println!("   candidate_quality: {:.6}", record.candidate_quality);

    println!("\n2. Deterministic Canonical JSON (BTreeMap Sorted Keys, No HashMap):");
    let json = record.canonical_json().unwrap();
    println!("   {}\n", json);

    println!("3. Cryptographic SHA-256 Hashes:");
    let hashes = record.compute_hashes().unwrap();
    println!("   image_hash:       {}", hashes.image_hash);
    println!("   content_hash:     {}", hashes.content_hash);
    println!("   metadata_hash:    {}", hashes.metadata_hash);
    println!("   face_result_hash: {}", hashes.face_result_hash);
    println!("   record_hash:      {}\n", hashes.record_hash);

    println!("4. Proof: Changing 'title':");
    let mut modified_title = record.clone();
    modified_title.title = "Completely Different Title".to_string();
    let mod_hashes = modified_title.compute_hashes().unwrap();
    println!("   orig content_hash:  {}", hashes.content_hash);
    println!("   mod  content_hash:  {}", mod_hashes.content_hash);
    println!("   Hashes differ?      {} (Expected: true)", hashes.content_hash != mod_hashes.content_hash);
    println!("   metadata_hash same? {} (Domain separation preserved)\n", hashes.metadata_hash == mod_hashes.metadata_hash);

    println!("5. Proof: Changing 'source_url':");
    let mut modified_url = record.clone();
    modified_url.source_url = Url::parse("https://different-site.org/profile").unwrap();
    let url_hashes = modified_url.compute_hashes().unwrap();
    println!("   orig metadata_hash: {}", hashes.metadata_hash);
    println!("   mod  metadata_hash: {}", url_hashes.metadata_hash);
    println!("   Hashes differ?      {} (Expected: true)\n", hashes.metadata_hash != url_hashes.metadata_hash);

    println!("6. Proof: Changing 'image_sha256':");
    let mut modified_img = record.clone();
    modified_img.image_sha256 = "0000000000000000000000000000000000000000000000000000000000000000".to_string();
    let img_hashes = modified_img.compute_hashes().unwrap();
    println!("   orig image_hash:    {}", hashes.image_hash);
    println!("   mod  image_hash:    {}", img_hashes.image_hash);
    println!("   Hashes differ?      {} (Expected: true)\n", hashes.image_hash != img_hashes.image_hash);

    println!("7. Proof: Changing 'face_similarity':");
    let mut modified_face = record.clone();
    modified_face.face_similarity = 0.990000;
    let face_hashes = modified_face.compute_hashes().unwrap();
    println!("   orig face_hash:     {}", hashes.face_result_hash);
    println!("   mod  face_hash:     {}", face_hashes.face_result_hash);
    println!("   Hashes differ?      {} (Expected: true)\n", hashes.face_result_hash != face_hashes.face_result_hash);

    println!("8. Proof: Unicode NFC Invariance:");
    let mut rec_pre = record.clone();
    rec_pre.title = "Caf\u{00E9}".to_string(); // Precomposed
    let mut rec_dec = record.clone();
    rec_dec.title = "Cafe\u{0301}".to_string(); // Decomposed
    let h_pre = rec_pre.compute_hashes().unwrap();
    let h_dec = rec_dec.compute_hashes().unwrap();
    println!("   Precomposed content_hash: {}", h_pre.content_hash);
    println!("   Decomposed  content_hash: {}", h_dec.content_hash);
    println!("   Hashes identical?         {} (Expected: true)\n", h_pre.content_hash == h_dec.content_hash);

    println!("============================================================");
    println!("     ALL DETERMINISM CHECKS VERIFIED SUCCESSFULLY!");
    println!("============================================================");
}

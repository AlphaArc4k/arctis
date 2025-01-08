

pub fn get_cid_from_url(ipfs_url: &str) -> Option<String> {
  let gateways = vec![
    "https://gateway.pinata.cloud/ipfs/",
    "https://ipfs.io/ipfs/",
    "https://cf-ipfs.com/ipfs/",
  ];
  for gateway in gateways {
    if ipfs_url.starts_with(gateway) {
      let cid = ipfs_url.replace(gateway, "");
      return Some(cid);
    }
  }
  let re = Regex::new(r"(Qm[1-9A-Za-z]{44})").unwrap();
  re.captures(ipfs_url)
  .and_then(|cap| cap.get(0).map(|cid| cid.as_str().to_string()))
}
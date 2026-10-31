use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Device {
    pub model: String,
    pub upgrade: u32,
    pub upgrade_plan: u32,
    pub aal: u32,
    pub aal_plan: u32,
    pub port: u32,
    pub port_plan: u32,
    pub new: u32,
    pub new_plan: u32,
    pub months: u32,
    pub notes: String,
}
#[derive(Clone, Copy, PartialEq)]
pub enum Transaction {
    New,
    Port,
    Add,
    AddPort,
    Upgrade,
}
pub fn money(cents: u32) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}
pub fn cents(s: &str) -> Result<u32, String> {
    let s = s.trim().trim_start_matches('$').replace(',', "");
    if s.eq_ignore_ascii_case("free") {
        return Ok(0);
    }
    let mut p = s.split('.');
    let whole = p
        .next()
        .unwrap_or("")
        .parse::<u32>()
        .map_err(|_| "Enter a positive dollar amount")?;
    let fraction = p.next().unwrap_or("0");
    if fraction.len() > 2 || !fraction.chars().all(|c| c.is_ascii_digit()) || p.next().is_some() {
        return Err("Use dollars and up to two cents digits".into());
    }
    whole
        .checked_mul(100)
        .and_then(|v| {
            v.checked_add(fraction.parse::<u32>().ok()? * if fraction.len() == 1 { 10 } else { 1 })
        })
        .ok_or("Amount too large".into())
}
pub fn quote(d: &Device, t: Transaction, accessories: u32, fees: u32) -> Result<String, String> {
    let (phone, required) = match t {
        Transaction::New => (d.new, d.new_plan),
        Transaction::Port => (d.port, d.port_plan),
        Transaction::Add => (d.aal, d.aal_plan),
        Transaction::AddPort => (if d.months > 1 { d.port } else { d.aal }, d.aal_plan),
        Transaction::Upgrade => (d.upgrade, d.upgrade_plan),
    };
    let plan = match t {
        Transaction::Upgrade => 0,
        Transaction::Add | Transaction::AddPort => required.saturating_sub(2000),
        _ => required,
    };
    let total = phone
        .checked_add(plan)
        .and_then(|v| v.checked_add(accessories))
        .and_then(|v| v.checked_add(fees))
        .ok_or("Quote too large")?;
    Ok(format!("Phone: {}\r\nRequired plan: {} / month\r\nPlan charged today: {}\r\nAccessories: {}\r\nTax / activation / other fees entered: {}\r\n\r\nESTIMATED TODAY: {}\r\n\r\n{}\r\n{}",money(phone),money(required),money(plan),money(accessories),money(fees),money(total),if t==Transaction::Upgrade {"Upgrade: plan excluded. Verify customer promo eligibility in Elevate / e-pay."} else if d.months>1 {"Two-month plan requirement: quote includes FIRST month only. Confirm collection and eligibility in promo deck."} else {"Confirm promo eligibility before sale. Taxes and fees are manual."},d.notes))
}
pub fn read_csv(text: &str) -> Result<Vec<Device>, String> {
    let devices: Vec<Device> = csv::Reader::from_reader(text.as_bytes())
        .deserialize()
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    if devices.is_empty() || devices.len() > 2000 {
        return Err("Catalog must contain 1–2000 phones".into());
    }
    for d in &devices {
        if d.model.is_empty() || d.months == 0 || d.months > 12 {
            return Err("Invalid model or month requirement".into());
        }
    }
    Ok(devices)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn d() -> Device {
        Device {
            model: "Test".into(),
            upgrade: 14999,
            upgrade_plan: 6000,
            aal: 3999,
            aal_plan: 6000,
            port: 0,
            port_plan: 6000,
            new: 3999,
            new_plan: 6000,
            months: 1,
            notes: String::new(),
        }
    }
    #[test]
    fn rules() {
        assert!(quote(&d(), Transaction::Upgrade, 1000, 0)
            .unwrap()
            .contains("$159.99"));
        assert!(quote(&d(), Transaction::Add, 1000, 0)
            .unwrap()
            .contains("$89.99"));
        assert!(quote(&d(), Transaction::New, 0, 0)
            .unwrap()
            .contains("$99.99"));
    }
    #[test]
    fn amounts() {
        assert_eq!(cents("14.9").unwrap(), 1490);
        assert!(cents("-1").is_err());
        assert!(cents("1.001").is_err());
        assert!(cents("4294967295").is_err());
    }
    #[test]
    fn promo_add_port() {
        let mut d = d();
        d.months = 2;
        assert!(quote(&d, Transaction::AddPort, 0, 0)
            .unwrap()
            .contains("ESTIMATED TODAY: $40.00"));
    }
}
/// Imports the primary phone table only. Other sections use different schemas.
pub fn import_pdf(path: &std::path::Path) -> Result<Vec<Device>, String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 8_000_000 {
        return Err("PDF exceeds 8 MB".into());
    }
    let text = pdf_extract::extract_text(path).map_err(|e| e.to_string())?;
    parse_pdf_text(&text)
}
pub fn parse_pdf_text(text: &str) -> Result<Vec<Device>, String> {
    if !text.contains("BOOST MOBILE - DEVICE HOTSHEET") || !text.contains("TABLETS AND WATCHES") {
        return Err("Unsupported PDF layout; use CSV".into());
    }
    let price = r"(\$[0-9,]+(?:\.[0-9]{2})?|[Ff][Rr][Ee][Ee])";
    let plan = r"(\$[0-9]+)(?:\s+x\s+2\s+mo)?";
    let pattern = format!(
        r"^(.+?)\s+\$[0-9,]+\.[0-9]{{2}}\s+{plan}\s+{price}\s+-\s+{plan}\s+({price}|see notes)\s+{plan}\s+{price}\s+{plan}\s+{price}(?:\s|$)"
    );
    let re = regex::Regex::new(&pattern).map_err(|e| e.to_string())?;
    let mut devices = Vec::new();
    for line in text.split("TABLETS AND WATCHES").next().unwrap().lines() {
        if let Some(c) = re.captures(line.trim()) {
            let value = |i: usize| cents(c.get(i).unwrap().as_str());
            let months = if line.contains("x 2 mo") { 2 } else { 1 };
            let new = value(10)?;
            let model = c[1]
                .trim()
                .strip_prefix("Google Google ")
                .map(|s| format!("Google {s}"))
                .unwrap_or_else(|| c[1].trim().into());
            devices.push(Device{model,upgrade_plan:value(2)?,upgrade:value(3)?,aal_plan:value(4)?,aal:if &c[5]=="see notes"{new}else{value(5)?},port_plan:value(7)?,port:value(8)?,new_plan:value(9)?,new,months,notes:"Imported primary phone table. Verify source effective date and eligibility. 12-month plan / 180-day port-out restrictions may apply. Special targeted, trade-in, No-ID and other tables excluded.".into()});
        }
    }
    if devices.is_empty() {
        return Err("No supported phone rows found".into());
    }
    Ok(devices)
}

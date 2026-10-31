# Boost Cat

A small, offline Rust quote assistant for Windows 10/11 (64-bit). Click the orange kitten above the taskbar to open the calculator. Close the calculator to hide it; right-click the kitten to quit.

## Run

Download the [portable Windows ZIP](downloads/BoostCat-Windows.zip), extract it, open the BoostCat folder, and double-click `BoostCat.exe`. On GitHub, open the ZIP and click **Download raw file**. You can also download `Boost-Cat-Windows` from the GitHub Actions build. No Rust installation, browser runtime, server, login, or API key is required on the store PC. The current version uses a static kitten with no idle animation or keyboard hook.

## Quote rules

- New number and port-in have separate phone prices and required plans.
- Upgrade: phone + accessories + manually entered fees. Plan charge is **$0**; required plan still appears for eligibility.
- Add a line: phone + max(required plan − $20, $0) + accessories + manually entered fees.
- For the three two-month AAL promotions in the sample, port-in uses the port offer; new-number uses the non-port offer, as stated in the notes.
- Two-month promotions show a reminder. Today’s estimate includes **one month**. Confirm whether both months must be collected using the promo deck before sale.
- Accessories is the combined dollar amount for accessories being sold. Tax is calculated automatically at **8.25%** after adding phone, charged plan and accessories: **subtotal × 1.0825**, rounded to the nearest cent. Activation and other manually entered fees are added after tax.
- All amounts use integer cents. Customer details are not collected.

## Hot sheets

32 phones from the **main phone table** of the provided sheet, effective October 30, 2026, are bundled. This is a dated sample, not a live pricing service.

Choose **Import sheet**, select a PDF or CSV, review the scrollable preview against the source, then choose **Accept import**. The previous catalog remains active until accepted. The accepted CSV is stored in `%LOCALAPPDATA%\BoostCat\catalog.csv`. Delete it to restore the bundled sample. Import files are limited to 8 MB.

PDF import supports the main phone-table layout demonstrated by the sample. It intentionally excludes EDLP, renewed-authorized-retailer, closeout, tablet/watch, Project Velocity, No-ID, trade-in, and targeted tier upgrade sections. Different layouts or scanned PDFs may require manual CSV entry. Imported catalogs can be partial: always review the row count and all prices. Eligibility, effective dates, 12-month plan restrictions, and 180-day port-out restrictions must be verified at sale. Special upgrade offers must be checked in Elevate/e-pay.

Use `data/sample.csv` as the CSV template. Monetary columns are **integer cents** (6000 = $60), `months` is the promotional plan duration, and `notes` carries conditions. Each model must occupy its own row.

## Memory design

Native Win32 controls and GDI rendering; no Electron, WebView, GPU framework, background polling, or permanent animation timer. The event loop blocks while idle. The cat is decoded once at 160 pixels. PDF parsing runs in a short-lived copy of the executable, returning parser allocations to Windows at exit. The parser temporarily increases memory usage and the calculator waits during import. Release builds use size optimization, LTO and stripping.

**RAM has not been measured on Windows.** Native UI behavior, DPI scaling, taskbar placement and actual working set need validation on the store PC. This build positions the kitten at the bottom-right of the primary monitor’s work area; it does not attach to Explorer or support custom taskbar docking.

## Build and test

Install stable Rust and the Visual Studio C++ build tools on a development PC, then:

```powershell
cargo test --locked
cargo build --release --locked
```

Copy `target\release\boost-cat.exe` to the store PC. The store PC does not need build tools. GitHub Actions also builds a portable executable on Windows.

Tests cover money validation, transaction rules, two-month AAL port pricing and all 32 sample PDF rows against an independently extracted CSV. Windows-target compilation is checked during development; execution on Windows is still required before relying on it at the counter.

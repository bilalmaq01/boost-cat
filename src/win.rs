use boost_cat::*;
use std::{cell::RefCell, ptr::null_mut};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::*,
    UI::{Controls::Dialogs::*, Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
struct App {
    devices: Vec<Device>,
    panel: HWND,
    cat: HWND,
    controls: Vec<HWND>,
    pixels: Vec<u8>,
    width: i32,
    height: i32,
    pending: Option<(String, Vec<Device>)>,
}
thread_local! {static APP:RefCell<Option<App>>=const{RefCell::new(None)};}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    id: usize,
    style: u32,
) -> HWND {
    let hwnd = CreateWindowExW(
        0,
        wide(class).as_ptr(),
        wide(text).as_ptr(),
        WS_CHILD | WS_VISIBLE | style,
        x,
        y,
        w,
        h,
        parent,
        id as _,
        GetModuleHandleW(std::ptr::null()),
        std::ptr::null(),
    );
    SendMessageW(
        hwnd,
        WM_SETFONT,
        GetStockObject(DEFAULT_GUI_FONT) as usize,
        1,
    );
    hwnd
}
unsafe fn text(h: HWND) -> String {
    let mut buf = vec![0u16; GetWindowTextLengthW(h) as usize + 1];
    let n = GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32);
    String::from_utf16_lossy(&buf[..n as usize])
}
unsafe fn message(h: HWND, s: &str) {
    MessageBoxW(h, wide(s).as_ptr(), wide("Boost Cat").as_ptr(), MB_OK);
}
fn path() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| ".".into()))
        .join("BoostCat")
        .join("catalog.csv")
}
unsafe fn populate(a: &App) {
    SendMessageW(a.controls[0], CB_RESETCONTENT, 0, 0);
    for d in &a.devices {
        SendMessageW(
            a.controls[0],
            CB_ADDSTRING,
            0,
            wide(&d.model).as_ptr() as isize,
        );
    }
    SendMessageW(a.controls[0], CB_SETCURSEL, 0, 0);
}
unsafe fn calculate(a: &App) {
    let i = SendMessageW(a.controls[0], CB_GETCURSEL, 0, 0) as usize;
    let t = match SendMessageW(a.controls[1], CB_GETCURSEL, 0, 0) {
        1 => Transaction::Port,
        2 => Transaction::Add,
        3 => Transaction::AddPort,
        4 => Transaction::Upgrade,
        _ => Transaction::New,
    };
    let result = (|| {
        let d = a.devices.get(i).ok_or("Select a phone")?;
        quote(
            d,
            t,
            cents(&text(a.controls[2]))?,
            cents(&text(a.controls[3]))?,
        )
    })();
    SetWindowTextW(
        a.controls[4],
        wide(&result.unwrap_or_else(|e| format!("Cannot quote: {e}"))).as_ptr(),
    );
}
unsafe fn pick_file(parent: HWND) -> Option<std::path::PathBuf> {
    let mut file = [0u16; 32768];
    let filter = wide("Hot sheets (*.pdf;*.csv)\0*.pdf;*.csv\0\0");
    let mut ofn: OPENFILENAMEW = std::mem::zeroed();
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = parent;
    ofn.lpstrFilter = filter.as_ptr();
    ofn.lpstrFile = file.as_mut_ptr();
    ofn.nMaxFile = file.len() as u32;
    ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
    if GetOpenFileNameW(&mut ofn) != 0 {
        Some(String::from_utf16_lossy(&file[..file.iter().position(|c| *c == 0).unwrap()]).into())
    } else {
        None
    }
}
unsafe fn import(a: &mut App) {
    let Some(file) = pick_file(a.panel) else {
        return;
    };
    if std::fs::metadata(&file)
        .map(|m| m.len() > 8_000_000)
        .unwrap_or(true)
    {
        message(a.panel, "Choose a hot sheet smaller than 8 MB.");
        return;
    }
    let result = if file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .eq_ignore_ascii_case("pdf")
    {
        // A short-lived worker returns all PDF parser allocations to the OS on exit.
        let tmp = path().with_extension("candidate.csv");
        let _ = std::fs::create_dir_all(tmp.parent().unwrap());
        use std::os::windows::process::CommandExt;
        std::process::Command::new(std::env::current_exe().unwrap()).arg("--extract").arg(&file).arg(&tmp).creation_flags(0x08000000).status().map_err(|e|e.to_string()).and_then(|s|if s.success(){std::fs::read_to_string(&tmp).map_err(|e|e.to_string())}else{Err("This PDF layout could not be read safely. Use the CSV template; no prices were changed.".into())})
    } else {
        std::fs::read_to_string(&file).map_err(|e| e.to_string())
    };
    match result.and_then(|s| read_csv(&s).map(|d| (s, d))) {
        Ok((s, d)) => {
            let summary=d.iter().map(|v|format!("{} | New {} / plan {} | Port {} / plan {} | AAL {} / plan {} | Upgrade {} / req {} | {} month(s)",v.model,money(v.new),money(v.new_plan),money(v.port),money(v.port_plan),money(v.aal),money(v.aal_plan),money(v.upgrade),money(v.upgrade_plan),v.months)).collect::<Vec<_>>().join("\r\n");
            SetWindowTextW(a.controls[4],wide(&format!("IMPORT PREVIEW — {} phones\r\nVerify against source sheet; click Accept import to save. Primary phone table only; special offers excluded.\r\n\r\n{}",d.len(),summary)).as_ptr());
            a.pending = Some((s, d));
            EnableWindow(GetDlgItem(a.panel, 13), 1);
        }
        Err(e) => message(a.panel, &e),
    }
}
unsafe fn position(cat: HWND) {
    let mut r: RECT = std::mem::zeroed();
    SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut r as *mut _ as _, 0);
    SetWindowPos(
        cat,
        HWND_TOPMOST,
        r.right - 200,
        r.bottom - 160,
        160,
        160,
        SWP_NOACTIVATE,
    );
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: usize, lp: isize) -> isize {
    match msg {
        WM_COMMAND => {
            APP.with(|cell| {
                let Ok(mut state) = cell.try_borrow_mut() else {
                    return;
                };
                if let Some(a) = state.as_mut() {
                    match wp & 0xffff {
                        10 => import(a),
                        13 => {
                            if let Some((s, d)) = a.pending.take() {
                                if let Err(e) = std::fs::create_dir_all(path().parent().unwrap())
                                    .and_then(|_| std::fs::write(path(), s))
                                {
                                    message(a.panel, &format!("Could not save catalog: {e}"));
                                    return;
                                }
                                a.devices = d;
                                populate(a);
                                calculate(a);
                                EnableWindow(GetDlgItem(a.panel, 13), 0);
                            }
                        }
                        11 => {
                            ShowWindow(a.panel, SW_HIDE);
                        }
                        12 => {
                            DestroyWindow(a.cat);
                        }
                        _ => {
                            if a.pending.is_none() {
                                calculate(a)
                            }
                        }
                    }
                }
            });
            0
        }
        WM_LBUTTONUP => {
            APP.with(|cell| {
                if let Some(a) = cell.try_borrow().ok().as_deref().and_then(|s| s.as_ref()) {
                    if hwnd == a.cat {
                        ShowWindow(a.panel, SW_SHOW);
                        SetForegroundWindow(a.panel);
                    }
                }
            });
            0
        }
        WM_RBUTTONUP => {
            let menu = CreatePopupMenu();
            AppendMenuW(menu, MF_STRING, 1, wide("Open calculator").as_ptr());
            AppendMenuW(menu, MF_STRING, 2, wide("Quit Boost Cat").as_ptr());
            let mut p: POINT = std::mem::zeroed();
            GetCursorPos(&mut p);
            SetForegroundWindow(hwnd);
            let choice = TrackPopupMenu(menu, TPM_RETURNCMD, p.x, p.y, 0, hwnd, std::ptr::null());
            DestroyMenu(menu);
            if choice == 2 {
                DestroyWindow(hwnd);
            } else if choice == 1 {
                APP.with(|c| {
                    if let Some(a) = c.try_borrow().ok().as_deref().and_then(|s| s.as_ref()) {
                        ShowWindow(a.panel, SW_SHOW);
                    }
                });
            }
            0
        }
        WM_DISPLAYCHANGE | WM_SETTINGCHANGE => {
            APP.with(|c| {
                if let Some(a) = c.try_borrow().ok().as_deref().and_then(|s| s.as_ref()) {
                    position(a.cat);
                }
            });
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let dc = BeginPaint(hwnd, &mut ps);
            APP.with(|c| {
                let Ok(state) = c.try_borrow() else { return };
                if let Some(a) = state.as_ref() {
                    if hwnd == a.cat {
                        let brush = CreateSolidBrush(0x00ff00ff);
                        FillRect(dc, &ps.rcPaint, brush);
                        DeleteObject(brush);
                        let mut info: BITMAPINFO = std::mem::zeroed();
                        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                        info.bmiHeader.biWidth = a.width;
                        info.bmiHeader.biHeight = -a.height;
                        info.bmiHeader.biPlanes = 1;
                        info.bmiHeader.biBitCount = 32;
                        StretchDIBits(
                            dc,
                            0,
                            0,
                            160,
                            160,
                            0,
                            0,
                            a.width,
                            a.height,
                            a.pixels.as_ptr() as _,
                            &info,
                            DIB_RGB_COLORS,
                            SRCCOPY,
                        );
                    }
                }
            });
            EndPaint(hwnd, &ps);
            0
        }
        WM_CLOSE => {
            let is_cat = APP.with(|c| {
                c.try_borrow()
                    .ok()
                    .as_deref()
                    .and_then(|s| s.as_ref())
                    .map(|a| hwnd == a.cat)
                    .unwrap_or(false)
            });
            if is_cat {
                DestroyWindow(hwnd);
            } else {
                ShowWindow(hwnd, SW_HIDE);
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
pub fn run() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() == 4 && args[1] == "--extract" {
        let result = boost_cat::import_pdf(std::path::Path::new(&args[2])).and_then(|d| {
            let mut w = csv::Writer::from_path(&args[3]).map_err(|e| e.to_string())?;
            for row in d {
                w.serialize(row).map_err(|e| e.to_string())?;
            }
            w.flush().map_err(|e| e.to_string())
        });
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let class = wide("BoostCatNative");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_WINDOW + 1) as _,
            ..std::mem::zeroed()
        };
        RegisterClassW(&wc);
        let cat = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            class.as_ptr(),
            wide("Boost Cat — click to quote; right-click to quit").as_ptr(),
            WS_POPUP,
            0,
            0,
            160,
            160,
            null_mut(),
            null_mut(),
            instance,
            std::ptr::null(),
        );
        SetLayeredWindowAttributes(cat, 0x00ff00ff, 255, LWA_COLORKEY);
        let panel = CreateWindowExW(
            0,
            class.as_ptr(),
            wide("Boost Cat · Phone quotes").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            580,
            665,
            null_mut(),
            null_mut(),
            instance,
            std::ptr::null(),
        );
        control(
            panel,
            "STATIC",
            "BOOST CAT   /   A little help at the counter",
            20,
            18,
            530,
            24,
            0,
            0,
        );
        control(
            panel,
            "STATIC",
            "Phone · bundled sheet 10/30/2026; verify imported effective date",
            20,
            52,
            530,
            20,
            0,
            0,
        );
        let phone = control(
            panel,
            "COMBOBOX",
            "",
            20,
            76,
            530,
            280,
            1,
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP,
        );
        control(panel, "STATIC", "Transaction", 20, 112, 530, 20, 0, 0);
        let trans = control(
            panel,
            "COMBOBOX",
            "",
            20,
            136,
            530,
            220,
            2,
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP,
        );
        for s in [
            "New line · new number",
            "New line · port-in",
            "Add a line · new number",
            "Add a line · port-in",
            "Upgrade · no plan charge",
        ] {
            SendMessageW(trans, CB_ADDSTRING, 0, wide(s).as_ptr() as isize);
        }
        SendMessageW(trans, CB_SETCURSEL, 0, 0);
        control(
            panel,
            "STATIC",
            "Accessories total ($)",
            20,
            174,
            245,
            20,
            0,
            0,
        );
        let acc = control(
            panel,
            "EDIT",
            "0.00",
            20,
            198,
            245,
            28,
            3,
            WS_BORDER | WS_TABSTOP,
        );
        control(
            panel,
            "STATIC",
            "Tax + activation + other fees ($)",
            285,
            174,
            265,
            20,
            0,
            0,
        );
        let fees = control(
            panel,
            "EDIT",
            "0.00",
            285,
            198,
            265,
            28,
            4,
            WS_BORDER | WS_TABSTOP,
        );
        let output = control(
            panel,
            "EDIT",
            "",
            20,
            246,
            530,
            278,
            5,
            ES_MULTILINE as u32
                | ES_READONLY as u32
                | ES_AUTOVSCROLL as u32
                | WS_VSCROLL
                | WS_BORDER,
        );
        control(
            panel,
            "BUTTON",
            "Import sheet",
            20,
            542,
            125,
            34,
            10,
            WS_TABSTOP,
        );
        let accept = control(
            panel,
            "BUTTON",
            "Accept import",
            155,
            542,
            125,
            34,
            13,
            WS_TABSTOP,
        );
        EnableWindow(accept, 0);
        control(panel, "BUTTON", "Hide", 290, 542, 125, 34, 11, WS_TABSTOP);
        control(panel, "BUTTON", "Quit", 425, 542, 125, 34, 12, WS_TABSTOP);
        control(
            panel,
            "STATIC",
            "Offline • Upgrade plan $0 • Add-a-line plan minus $20",
            20,
            592,
            530,
            22,
            0,
            0,
        );
        let decoder = png::Decoder::new(std::io::Cursor::new(include_bytes!("../assets/cat.png")));
        let mut reader = decoder.read_info().expect("embedded cat");
        let mut pixels = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut pixels).unwrap();
        pixels.truncate(info.buffer_size());
        for p in pixels.chunks_exact_mut(4) {
            if p[3] < 128 {
                p.copy_from_slice(&[255, 0, 255, 255]);
            } else {
                p.swap(0, 2);
            }
        }
        let devices = std::fs::read_to_string(path())
            .ok()
            .and_then(|s| read_csv(&s).ok())
            .unwrap_or_else(|| {
                read_csv(include_str!("../data/sample.csv")).expect("embedded catalog")
            });
        let a = App {
            devices,
            panel,
            cat,
            controls: vec![phone, trans, acc, fees, output],
            pixels,
            width: info.width as i32,
            height: info.height as i32,
            pending: None,
        };
        populate(&a);
        calculate(&a);
        APP.with(|c| *c.borrow_mut() = Some(a));
        position(cat);
        ShowWindow(cat, SW_SHOWNOACTIVATE);
        ShowWindow(panel, SW_SHOW);
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            if IsDialogMessageW(panel, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

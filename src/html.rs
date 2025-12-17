use std::collections::HashMap;
use time;

pub fn generate_temperature_page(temps: &HashMap<String, Option<f32>>) -> String {
    let mut rows = String::new();
    let mut temp_vec: Vec<_> = temps.iter().collect();
    temp_vec.sort_by_key(|(name, _)| name.as_str());

    for (name, temp) in temp_vec {
        let temp_display = match temp {
            Some(t) => {
                let color = if *t < 20.0 {
                    "#88C0D0" // nord8 - frost blue
                } else if *t < 25.0 {
                    "#A3BE8C" // nord14 - aurora green
                } else if *t < 30.0 {
                    "#EBCB8B" // nord13 - aurora yellow
                } else {
                    "#BF616A" // nord11 - aurora red
                };
                format!(
                    "<span style='color: {}; font-size: 2em; font-weight: bold;'>{:.1}°C</span>",
                    color, t
                )
            }
            None => "<span style='color: #D08770; font-style: italic;'>Error</span>".to_string(),
        };

        rows.push_str(&format!(
            "<tr><td style='padding: 15px; border-bottom: 1px solid #4C566A;'>{}</td>\
             <td style='padding: 15px; border-bottom: 1px solid #4C566A; text-align: right;'>{}</td></tr>",
            name, temp_display
        ));
    }

    let datetime = time::OffsetDateTime::now_utc();

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta http-equiv="refresh" content="15">
    <title>Temperature Monitor</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            max-width: 800px;
            margin: 40px auto;
            padding: 20px;
            background: #3B4252;
            color: #ECEFF4;
        }}
        .container {{
            background: #2E3440;
            border-radius: 8px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.3);
            padding: 30px;
        }}
        h1 {{
            color: #ECEFF4;
            margin-top: 0;
            border-bottom: 3px solid #88C0D0;
            padding-bottom: 10px;
        }}
        table {{
            width: 100%;
            border-collapse: collapse;
            margin-top: 20px;
        }}
        th {{
            text-align: left;
            padding: 15px;
            background: #434C5E;
            color: #ECEFF4;
            font-weight: 600;
        }}
        td {{
            color: #D8DEE9;
        }}
        .footer {{
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #4C566A;
            color: #D8DEE9;
            font-size: 0.9em;
        }}
        .footer a {{
            color: #88C0D0;
            text-decoration: none;
        }}
        .footer a:hover {{
            color: #81A1C1;
            text-decoration: underline;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🌡️ Temperature Monitor</h1>
        <table>
            <thead>
                <tr>
                    <th>Probe</th>
                    <th style="text-align: right;">Temperature</th>
                </tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>
        <div class="footer">
            Last updated: {} UTC (auto-refresh every 15s)<br>
            <a href="/metrics">Prometheus Metrics</a> | <a href="/health">Health Check</a>
        </div>
    </div>
</body>
</html>"#,
        rows, datetime
    )
}

//! HTML templates for the Mock IdP login pages.

use calendsync_core::auth::OidcProvider;

/// Escape HTML special characters to prevent XSS.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Generate an auto-submitting form POST page for Apple callback.
///
/// Apple uses `response_mode=form_post`, which sends the authorization code
/// and state as a form POST rather than URL query parameters. This template
/// mimics that behavior by rendering a form that auto-submits on page load.
pub fn form_post_page(redirect_uri: &str, code: &str, state: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Redirecting...</title>
</head>
<body onload="document.forms[0].submit()">
    <form method="POST" action="{redirect_uri}">
        <input type="hidden" name="code" value="{code}" />
        <input type="hidden" name="state" value="{state}" />
        <noscript>
            <p>JavaScript is disabled. Click the button below to continue.</p>
            <button type="submit">Continue</button>
        </noscript>
    </form>
</body>
</html>"#,
        redirect_uri = html_escape(redirect_uri),
        code = html_escape(code),
        state = html_escape(state),
    )
}

/// Generate the HTML login page for a given OIDC provider.
pub fn login_page(provider: OidcProvider, state: &str, redirect_uri: &str) -> String {
    let provider_name = match provider {
        OidcProvider::Google => "Google",
        OidcProvider::Apple => "Apple",
    };

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Mock {provider_name} Sign In (DEV ONLY)</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, sans-serif;
            max-width: 400px;
            margin: 100px auto;
            padding: 20px;
        }}
        .warning {{
            background: #fff3cd;
            border: 1px solid #ffc107;
            padding: 15px;
            border-radius: 8px;
            margin-bottom: 20px;
        }}
        .warning h2 {{
            color: #856404;
            margin-top: 0;
        }}
        form {{
            background: #f8f9fa;
            padding: 20px;
            border-radius: 8px;
        }}
        label {{
            display: block;
            margin-bottom: 5px;
            font-weight: 500;
        }}
        input[type="email"], input[type="text"] {{
            width: 100%;
            padding: 10px;
            margin-bottom: 15px;
            border: 1px solid #ced4da;
            border-radius: 4px;
            box-sizing: border-box;
        }}
        button {{
            width: 100%;
            padding: 12px;
            background: #007bff;
            color: white;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 16px;
        }}
        button:hover {{
            background: #0056b3;
        }}
    </style>
</head>
<body>
    <div class="warning">
        <h2>Development Only</h2>
        <p>This is a <strong>mock {provider_name} login</strong> for development purposes.</p>
        <p>Enter any email address to simulate authentication.</p>
    </div>

    <form action="/authorize/submit" method="POST">
        <input type="hidden" name="provider" value="{}" />
        <input type="hidden" name="state" value="{state}" />
        <input type="hidden" name="redirect_uri" value="{redirect_uri}" />

        <label for="email">Email Address</label>
        <input type="email" id="email" name="email" placeholder="dev@example.com" required />

        <label for="name">Name (optional)</label>
        <input type="text" id="name" name="name" placeholder="Dev User" />

        <button type="submit">Sign in with {provider_name}</button>
    </form>
</body>
</html>"#,
        provider
    )
}

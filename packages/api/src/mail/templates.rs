use chrono::Utc;

fn base_template(content: &str, footer_text: &str) -> String {
    let year = Utc::now().format("%Y");
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta http-equiv="X-UA-Compatible" content="IE=edge">
    <title>Flow-Like</title>
    <!--[if mso]>
    <noscript>
        <xml>
            <o:OfficeDocumentSettings>
                <o:PixelsPerInch>96</o:PixelsPerInch>
            </o:OfficeDocumentSettings>
        </xml>
    </noscript>
    <![endif]-->
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap');
    </style>
</head>
<body style="margin: 0; padding: 0; font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; background-color: #050505; color: #ffffff; -webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale;">
    <table role="presentation" style="width: 100%; border-collapse: collapse; background-color: #050505;">
        <tr>
            <td style="padding: 48px 24px;">
                <table role="presentation" style="max-width: 560px; margin: 0 auto; background: #0a0a0a; border-radius: 24px; overflow: hidden; border: 1px solid #1a1a1a; box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);">
                    <!-- Decorative Top Bar -->
                    <tr>
                        <td style="height: 4px; background: linear-gradient(90deg, #3b82f6 0%, #8b5cf6 50%, #ec4899 100%);"></td>
                    </tr>

                    <!-- Logo Header -->
                    <tr>
                        <td style="padding: 40px 48px 24px; text-align: center;">
                            <table role="presentation" style="margin: 0 auto;">
                                <tr>
                                    <td style="background: linear-gradient(135deg, #3b82f6 0%, #8b5cf6 100%); padding: 14px 24px; border-radius: 16px; box-shadow: 0 10px 40px -10px rgba(59, 130, 246, 0.5);">
                                        <span style="font-size: 22px; font-weight: 700; color: white; letter-spacing: -0.5px;">✦ Flow-Like</span>
                                    </td>
                                </tr>
                            </table>
                        </td>
                    </tr>

                    <!-- Main Content -->
                    {content}

                    <!-- Footer -->
                    <tr>
                        <td style="padding: 32px 48px; background: #050505; border-top: 1px solid #1a1a1a;">
                            <table role="presentation" style="width: 100%;">
                                <tr>
                                    <td style="text-align: center; padding-bottom: 16px;">
                                        <p style="margin: 0 0 8px; font-size: 13px; color: #525252; line-height: 1.6;">
                                            {footer_text}
                                        </p>
                                        <p style="margin: 0; font-size: 13px; color: #525252;">
                                            Questions? Contact us at
                                            <a href="mailto:help@great-co.de" style="color: #3b82f6; text-decoration: none; font-weight: 500;">help@great-co.de</a>
                                        </p>
                                    </td>
                                </tr>
                                <tr>
                                    <td style="text-align: center; padding-top: 16px; border-top: 1px solid #1a1a1a;">
                                        <p style="margin: 0; font-size: 11px; color: #404040;">
                                            © {year} Flow-Like by Great Company. All rights reserved.
                                        </p>
                                    </td>
                                </tr>
                            </table>
                        </td>
                    </tr>
                </table>
            </td>
        </tr>
    </table>
</body>
</html>"##,
        content = content,
        footer_text = footer_text,
        year = year
    )
}

fn cta_button(text: &str, url: &str) -> String {
    format!(
        r#"<table role="presentation" style="margin: 0 auto;">
            <tr>
                <td style="background: linear-gradient(135deg, #3b82f6 0%, #8b5cf6 100%); border-radius: 14px; box-shadow: 0 8px 32px -8px rgba(59, 130, 246, 0.5);">
                    <a href="{url}" style="display: inline-block; color: white; text-decoration: none; font-size: 15px; font-weight: 600; padding: 16px 36px; letter-spacing: 0.2px;">
                        {text}
                    </a>
                </td>
            </tr>
        </table>"#,
        text = text,
        url = url
    )
}

pub fn status_badge(status: &str) -> String {
    let (bg_color, text_color, label) = match status.to_uppercase().as_str() {
        "AWAITING_DEPOSIT" => ("#fef3c7", "#92400e", "⏳ Awaiting Deposit"),
        "PENDING_REVIEW" => ("#e0e7ff", "#3730a3", "📋 Pending Review"),
        "IN_QUEUE" => ("#dbeafe", "#1e40af", "📥 In Queue"),
        "ONBOARDING_DONE" => ("#d1fae5", "#065f46", "✅ Onboarding Complete"),
        "IN_PROGRESS" => ("#fef3c7", "#92400e", "🔨 In Progress"),
        "DELIVERED" => ("#d1fae5", "#065f46", "🎉 Delivered"),
        "AWAITING_PAYMENT" => ("#fef3c7", "#92400e", "💳 Awaiting Payment"),
        "PAID" => ("#d1fae5", "#065f46", "✓ Paid"),
        "CANCELLED" => ("#fee2e2", "#991b1b", "✕ Cancelled"),
        "REFUNDED" => ("#e5e7eb", "#374151", "↩ Refunded"),
        _ => ("#e5e7eb", "#374151", status),
    };

    format!(
        r#"<span style="display: inline-block; background: {bg}; color: {color}; font-size: 13px; font-weight: 600; padding: 8px 16px; border-radius: 24px;">{label}</span>"#,
        bg = bg_color,
        color = text_color,
        label = label
    )
}

fn priority_badge() -> &'static str {
    r#"<span style="display: inline-block; background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%); color: white; font-size: 11px; font-weight: 600; padding: 4px 10px; border-radius: 12px; margin-left: 8px; vertical-align: middle;">⚡ PRIORITY</span>"#
}

fn info_card(title: &str, items: Vec<(&str, String)>) -> String {
    let items_html: String = items
        .iter()
        .map(|(label, value)| {
            format!(
                r#"<tr>
                    <td style="padding: 12px 0; border-bottom: 1px solid #1a1a1a;">
                        <span style="color: #737373; font-size: 13px;">{label}</span>
                    </td>
                    <td style="padding: 12px 0; border-bottom: 1px solid #1a1a1a; text-align: right;">
                        <span style="color: #ffffff; font-size: 13px; font-weight: 500;">{value}</span>
                    </td>
                </tr>"#,
                label = label,
                value = value
            )
        })
        .collect();

    format!(
        r#"<table role="presentation" style="width: 100%; background: #111111; border: 1px solid #1a1a1a; border-radius: 16px; margin-bottom: 24px;">
            <tr>
                <td colspan="2" style="padding: 20px 24px 12px;">
                    <h3 style="margin: 0; font-size: 12px; font-weight: 600; color: #525252; text-transform: uppercase; letter-spacing: 1px;">{title}</h3>
                </td>
            </tr>
            <tr>
                <td colspan="2" style="padding: 0 24px 20px;">
                    <table role="presentation" style="width: 100%;">
                        {items_html}
                    </table>
                </td>
            </tr>
        </table>"#,
        title = title,
        items_html = items_html
    )
}

pub fn solution_submission_confirmation(
    company_name: &str,
    tracking_url: &str,
    tracking_token: &str,
    pricing_tier: &str,
    is_priority: bool,
) -> (String, String) {
    let tier_display = match pricing_tier {
        "standard" => "Standard",
        "appstore" => "App Store",
        _ => pricing_tier,
    };

    let priority_html = if is_priority { priority_badge() } else { "" };

    let content = format!(
        r##"<tr>
            <td style="padding: 0 48px 32px; text-align: center;">
                <h1 style="margin: 0 0 12px; font-size: 28px; font-weight: 700; color: #ffffff; line-height: 1.3; letter-spacing: -0.5px;">
                    Request Received! 🎉
                </h1>
                <p style="margin: 0; font-size: 16px; color: #737373; line-height: 1.6;">
                    Your 24-Hour Solution request is on its way
                </p>
            </td>
        </tr>
        <tr>
            <td style="padding: 0 48px 40px;">
                <p style="margin: 0 0 24px; font-size: 15px; line-height: 1.7; color: #a3a3a3;">
                    Thank you for submitting your request for <strong style="color: #ffffff;">{company_name}</strong>.
                    We're excited to help bring your automation vision to life!
                </p>

                {info_card}

                <div style="text-align: center; margin-bottom: 32px;">
                    {cta_button}
                </div>

                <!-- Timeline -->
                <div style="background: linear-gradient(135deg, rgba(59, 130, 246, 0.08) 0%, rgba(139, 92, 246, 0.08) 100%); border: 1px solid rgba(59, 130, 246, 0.15); border-radius: 16px; padding: 28px;">
                    <h3 style="margin: 0 0 20px; font-size: 15px; font-weight: 600; color: #ffffff;">
                        What Happens Next?
                    </h3>
                    <table role="presentation" style="width: 100%;">
                        <tr>
                            <td style="vertical-align: top; padding-right: 16px; width: 32px;">
                                <div style="width: 28px; height: 28px; background: #1e40af; border-radius: 50%; text-align: center; line-height: 28px; font-size: 13px; font-weight: 600; color: white;">1</div>
                            </td>
                            <td style="padding-bottom: 20px;">
                                <p style="margin: 0 0 4px; font-size: 14px; font-weight: 600; color: #ffffff;">Review</p>
                                <p style="margin: 0; font-size: 13px; color: #737373;">Our team reviews your request within a few hours</p>
                            </td>
                        </tr>
                        <tr>
                            <td style="vertical-align: top; padding-right: 16px;">
                                <div style="width: 28px; height: 28px; background: #5b21b6; border-radius: 50%; text-align: center; line-height: 28px; font-size: 13px; font-weight: 600; color: white;">2</div>
                            </td>
                            <td style="padding-bottom: 20px;">
                                <p style="margin: 0 0 4px; font-size: 14px; font-weight: 600; color: #ffffff;">Onboarding Call</p>
                                <p style="margin: 0; font-size: 13px; color: #737373;">Quick call to clarify any details</p>
                            </td>
                        </tr>
                        <tr>
                            <td style="vertical-align: top; padding-right: 16px;">
                                <div style="width: 28px; height: 28px; background: #7c3aed; border-radius: 50%; text-align: center; line-height: 28px; font-size: 13px; font-weight: 600; color: white;">3</div>
                            </td>
                            <td style="padding-bottom: 20px;">
                                <p style="margin: 0 0 4px; font-size: 14px; font-weight: 600; color: #ffffff;">Development</p>
                                <p style="margin: 0; font-size: 13px; color: #737373;">Your solution built within 24 hours</p>
                            </td>
                        </tr>
                        <tr>
                            <td style="vertical-align: top; padding-right: 16px;">
                                <div style="width: 28px; height: 28px; background: linear-gradient(135deg, #3b82f6, #8b5cf6); border-radius: 50%; text-align: center; line-height: 28px; font-size: 13px; font-weight: 600; color: white;">✓</div>
                            </td>
                            <td>
                                <p style="margin: 0 0 4px; font-size: 14px; font-weight: 600; color: #ffffff;">Delivery</p>
                                <p style="margin: 0; font-size: 13px; color: #737373;">Receive your complete, working solution</p>
                            </td>
                        </tr>
                    </table>
                </div>
            </td>
        </tr>"##,
        company_name = company_name,
        info_card = info_card(
            "Request Details",
            vec![
                ("Plan", format!("{}{}", tier_display, priority_html)),
                (
                    "Tracking Token",
                    format!(
                        "<code style=\"background: #1a1a1a; padding: 4px 8px; border-radius: 6px; font-family: 'SF Mono', Monaco, monospace; font-size: 12px; color: #3b82f6;\">{}</code>",
                        tracking_token
                    )
                ),
            ]
        ),
        cta_button = cta_button("Track Your Request →", tracking_url)
    );

    let html = base_template(
        &content,
        "You're receiving this email because you submitted a 24-Hour Solution request.",
    );

    let text = format!(
        r#"YOUR 24-HOUR SOLUTION REQUEST HAS BEEN RECEIVED! 🎉

Hi there,

Thank you for submitting your 24-Hour Solution request for {company_name}. We're excited to help bring your automation vision to life!

REQUEST DETAILS
───────────────
Plan: {tier_display}{priority}
Tracking Token: {tracking_token}

Track your request: {tracking_url}

WHAT HAPPENS NEXT?
──────────────────
1. REVIEW — Our team will review your request within the next few hours
2. ONBOARDING CALL — We'll schedule a quick call to clarify any details
3. DEVELOPMENT — Your solution will be built within 24 hours of onboarding
4. DELIVERY — You'll receive your complete, working solution

Questions? Contact us at help@great-co.de

© {year} Flow-Like by Great Company. All rights reserved.
"#,
        company_name = company_name,
        tier_display = tier_display,
        priority = if is_priority { " (⚡ Priority)" } else { "" },
        tracking_token = tracking_token,
        tracking_url = tracking_url,
        year = Utc::now().format("%Y")
    );

    (html, text)
}

pub fn solution_status_update(
    company_name: &str,
    tracking_url: &str,
    old_status: &str,
    new_status: &str,
    message: Option<&str>,
) -> (String, String) {
    let emoji = match new_status.to_uppercase().as_str() {
        "PENDING_REVIEW" => "📋",
        "IN_QUEUE" => "📥",
        "ONBOARDING_DONE" => "✅",
        "IN_PROGRESS" => "🔨",
        "DELIVERED" => "🎉",
        "AWAITING_PAYMENT" => "💳",
        "PAID" => "✓",
        "CANCELLED" => "✕",
        "REFUNDED" => "↩",
        _ => "📬",
    };

    let headline = match new_status.to_uppercase().as_str() {
        "IN_QUEUE" => "You're In the Queue!",
        "ONBOARDING_DONE" => "Onboarding Complete!",
        "IN_PROGRESS" => "Work Has Started!",
        "DELIVERED" => "Your Solution is Ready! 🎉",
        "AWAITING_PAYMENT" => "Payment Required",
        "PAID" => "Payment Received!",
        "CANCELLED" => "Request Cancelled",
        "REFUNDED" => "Refund Processed",
        _ => "Status Update",
    };

    let description = match new_status.to_uppercase().as_str() {
        "IN_QUEUE" => "Your request has been approved and added to our development queue.",
        "ONBOARDING_DONE" => {
            "We've completed the onboarding process and are ready to start building your solution."
        }
        "IN_PROGRESS" => {
            "Our team is actively working on your solution. We'll notify you when it's ready!"
        }
        "DELIVERED" => {
            "Great news! Your solution has been completed and delivered. Check your tracking page for details."
        }
        "AWAITING_PAYMENT" => {
            "Your solution is ready. Please complete the final payment to receive access."
        }
        "PAID" => "Thank you for your payment! Your solution is fully complete.",
        "CANCELLED" => "Your request has been cancelled. If you have questions, please contact us.",
        "REFUNDED" => {
            "Your refund has been processed. The funds should appear in your account within 5-10 business days."
        }
        _ => "Your request status has been updated.",
    };

    let message_html = message
        .map(|m| {
            format!(
                r#"<div style="background: #111111; border-left: 3px solid #3b82f6; border-radius: 0 12px 12px 0; padding: 20px 24px; margin-bottom: 24px;">
            <p style="margin: 0 0 8px; font-size: 12px; font-weight: 600; color: #525252; text-transform: uppercase; letter-spacing: 0.5px;">Message from our team</p>
            <p style="margin: 0; font-size: 14px; color: #d4d4d4; line-height: 1.6;">{}</p>
        </div>"#,
                m
            )
        })
        .unwrap_or_default();

    let content = format!(
        r##"<tr>
            <td style="padding: 0 48px 32px; text-align: center;">
                <div style="font-size: 48px; margin-bottom: 16px;">{emoji}</div>
                <h1 style="margin: 0 0 12px; font-size: 26px; font-weight: 700; color: #ffffff; line-height: 1.3; letter-spacing: -0.5px;">
                    {headline}
                </h1>
                <p style="margin: 0; font-size: 15px; color: #737373; line-height: 1.6;">
                    Update for <strong style="color: #a3a3a3;">{company_name}</strong>
                </p>
            </td>
        </tr>
        <tr>
            <td style="padding: 0 48px 40px;">
                <!-- Status Change Card -->
                <div style="background: #111111; border: 1px solid #1a1a1a; border-radius: 16px; padding: 24px; margin-bottom: 24px;">
                    <table role="presentation" style="width: 100%;">
                        <tr>
                            <td style="width: 45%; text-align: center; vertical-align: middle;">
                                <p style="margin: 0 0 8px; font-size: 11px; color: #525252; text-transform: uppercase; letter-spacing: 1px;">Previous</p>
                                {old_badge}
                            </td>
                            <td style="width: 10%; text-align: center; vertical-align: middle;">
                                <span style="font-size: 20px; color: #404040;">→</span>
                            </td>
                            <td style="width: 45%; text-align: center; vertical-align: middle;">
                                <p style="margin: 0 0 8px; font-size: 11px; color: #525252; text-transform: uppercase; letter-spacing: 1px;">Current</p>
                                {new_badge}
                            </td>
                        </tr>
                    </table>
                </div>

                <p style="margin: 0 0 24px; font-size: 15px; line-height: 1.7; color: #a3a3a3;">
                    {description}
                </p>

                {message_html}

                <div style="text-align: center;">
                    {cta_button}
                </div>
            </td>
        </tr>"##,
        emoji = emoji,
        headline = headline,
        company_name = company_name,
        old_badge = status_badge(old_status),
        new_badge = status_badge(new_status),
        description = description,
        message_html = message_html,
        cta_button = cta_button("View Full Status →", tracking_url)
    );

    let html = base_template(
        &content,
        "You're receiving this because your 24-Hour Solution request status changed.",
    );

    let text = format!(
        r#"{emoji} {headline}

Update for {company_name}

STATUS CHANGE
─────────────
{old_status_display} → {new_status_display}

{description}
{message_section}
Track your request: {tracking_url}

Questions? Contact us at help@great-co.de

© {year} Flow-Like by Great Company. All rights reserved.
"#,
        emoji = emoji,
        headline = headline,
        company_name = company_name,
        old_status_display = old_status.replace('_', " "),
        new_status_display = new_status.replace('_', " "),
        description = description,
        message_section = message
            .map(|m| format!("\nMessage from our team:\n{}\n", m))
            .unwrap_or_default(),
        tracking_url = tracking_url,
        year = Utc::now().format("%Y")
    );

    (html, text)
}

pub fn solution_log_added(
    company_name: &str,
    tracking_url: &str,
    action: &str,
    details: Option<&str>,
    current_status: &str,
) -> (String, String) {
    let details_html = details
        .map(|d| {
            format!(
                r#"<div style="background: #0a0a0a; border: 1px solid #1a1a1a; border-radius: 12px; padding: 20px; margin-bottom: 24px;">
            <p style="margin: 0; font-size: 14px; color: #d4d4d4; line-height: 1.7; white-space: pre-wrap;">{}</p>
        </div>"#,
                d
            )
        })
        .unwrap_or_default();

    let content = format!(
        r##"<tr>
            <td style="padding: 0 48px 32px; text-align: center;">
                <div style="font-size: 48px; margin-bottom: 16px;">📝</div>
                <h1 style="margin: 0 0 12px; font-size: 26px; font-weight: 700; color: #ffffff; line-height: 1.3; letter-spacing: -0.5px;">
                    New Update
                </h1>
                <p style="margin: 0; font-size: 15px; color: #737373; line-height: 1.6;">
                    For <strong style="color: #a3a3a3;">{company_name}</strong>
                </p>
            </td>
        </tr>
        <tr>
            <td style="padding: 0 48px 40px;">
                <!-- Update Card -->
                <div style="background: linear-gradient(135deg, rgba(59, 130, 246, 0.08) 0%, rgba(139, 92, 246, 0.08) 100%); border: 1px solid rgba(59, 130, 246, 0.15); border-radius: 16px; padding: 24px; margin-bottom: 24px;">
                    <p style="margin: 0 0 8px; font-size: 12px; font-weight: 600; color: #525252; text-transform: uppercase; letter-spacing: 0.5px;">Update</p>
                    <p style="margin: 0; font-size: 18px; font-weight: 600; color: #ffffff; line-height: 1.4;">{action}</p>
                </div>

                {details_html}

                <div style="background: #111111; border: 1px solid #1a1a1a; border-radius: 12px; padding: 16px 20px; margin-bottom: 24px;">
                    <table role="presentation" style="width: 100%;">
                        <tr>
                            <td style="vertical-align: middle;">
                                <span style="font-size: 13px; color: #737373;">Current Status:</span>
                            </td>
                            <td style="text-align: right; vertical-align: middle;">
                                {status_badge}
                            </td>
                        </tr>
                    </table>
                </div>

                <div style="text-align: center;">
                    {cta_button}
                </div>
            </td>
        </tr>"##,
        company_name = company_name,
        action = action,
        details_html = details_html,
        status_badge = status_badge(current_status),
        cta_button = cta_button("View Full History →", tracking_url)
    );

    let html = base_template(
        &content,
        "You're receiving this because there's a new update on your 24-Hour Solution request.",
    );

    let text = format!(
        r#"📝 NEW UPDATE

For {company_name}

UPDATE
──────
{action}
{details_section}
Current Status: {status}

Track your request: {tracking_url}

Questions? Contact us at help@great-co.de

© {year} Flow-Like by Great Company. All rights reserved.
"#,
        company_name = company_name,
        action = action,
        details_section = details.map(|d| format!("\n{}\n", d)).unwrap_or_default(),
        status = current_status.replace('_', " "),
        tracking_url = tracking_url,
        year = Utc::now().format("%Y")
    );

    (html, text)
}

pub fn solution_delivered(
    company_name: &str,
    tracking_url: &str,
    delivery_notes: Option<&str>,
) -> (String, String) {
    let notes_html = delivery_notes
        .map(|n| {
            format!(
                r#"<div style="background: #111111; border-left: 3px solid #10b981; border-radius: 0 12px 12px 0; padding: 20px 24px; margin-bottom: 24px;">
            <p style="margin: 0 0 8px; font-size: 12px; font-weight: 600; color: #525252; text-transform: uppercase; letter-spacing: 0.5px;">Delivery Notes</p>
            <p style="margin: 0; font-size: 14px; color: #d4d4d4; line-height: 1.6; white-space: pre-wrap;">{}</p>
        </div>"#,
                n
            )
        })
        .unwrap_or_default();

    let content = format!(
        r##"<tr>
            <td style="padding: 0 48px 32px; text-align: center;">
                <div style="font-size: 56px; margin-bottom: 16px;">🎉</div>
                <h1 style="margin: 0 0 12px; font-size: 30px; font-weight: 700; color: #ffffff; line-height: 1.3; letter-spacing: -0.5px;">
                    Your Solution is Ready!
                </h1>
                <p style="margin: 0; font-size: 16px; color: #737373; line-height: 1.6;">
                    Congratulations, <strong style="color: #a3a3a3;">{company_name}</strong>!
                </p>
            </td>
        </tr>
        <tr>
            <td style="padding: 0 48px 40px;">
                <!-- Success Banner -->
                <div style="background: linear-gradient(135deg, rgba(16, 185, 129, 0.1) 0%, rgba(6, 95, 70, 0.1) 100%); border: 1px solid rgba(16, 185, 129, 0.2); border-radius: 16px; padding: 28px; margin-bottom: 24px; text-align: center;">
                    <p style="margin: 0 0 8px; font-size: 14px; color: #10b981; font-weight: 600;">✓ DELIVERED</p>
                    <p style="margin: 0; font-size: 15px; color: #a3a3a3; line-height: 1.6;">
                        Your 24-Hour Solution has been completed and delivered.<br>
                        Check your tracking page for all the details!
                    </p>
                </div>

                {notes_html}

                <div style="text-align: center; margin-bottom: 24px;">
                    {cta_button}
                </div>

                <p style="margin: 0; font-size: 14px; color: #737373; text-align: center; line-height: 1.6;">
                    Thank you for choosing Flow-Like. We hope this solution helps transform your workflow!
                </p>
            </td>
        </tr>"##,
        company_name = company_name,
        notes_html = notes_html,
        cta_button = cta_button("Access Your Solution →", tracking_url)
    );

    let html = base_template(&content, "Your 24-Hour Solution has been delivered!");

    let text = format!(
        r#"🎉 YOUR SOLUTION IS READY!

Congratulations, {company_name}!

Your 24-Hour Solution has been completed and delivered.
{notes_section}
Access your solution: {tracking_url}

Thank you for choosing Flow-Like. We hope this solution helps transform your workflow!

Questions? Contact us at help@great-co.de

© {year} Flow-Like by Great Company. All rights reserved.
"#,
        company_name = company_name,
        notes_section = delivery_notes
            .map(|n| format!("\nDelivery Notes:\n{}\n", n))
            .unwrap_or_default(),
        tracking_url = tracking_url,
        year = Utc::now().format("%Y")
    );

    (html, text)
}

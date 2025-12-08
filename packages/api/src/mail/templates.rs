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

    let priority_badge = if is_priority {
        r#"<span style="display: inline-block; background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%); color: white; font-size: 12px; font-weight: 600; padding: 4px 12px; border-radius: 20px; margin-left: 8px;">⚡ Priority</span>"#
    } else {
        ""
    };

    let html = format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Your 24-Hour Solution Request</title>
</head>
<body style="margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; background-color: #0a0a0a; color: #ffffff;">
    <table role="presentation" style="width: 100%; border-collapse: collapse;">
        <tr>
            <td style="padding: 40px 20px;">
                <table role="presentation" style="max-width: 600px; margin: 0 auto; background: linear-gradient(180deg, #171717 0%, #0a0a0a 100%); border-radius: 16px; overflow: hidden; border: 1px solid #262626;">
                    <!-- Header -->
                    <tr>
                        <td style="padding: 40px 40px 20px; text-align: center; border-bottom: 1px solid #262626;">
                            <div style="display: inline-block; background: linear-gradient(135deg, #3b82f6 0%, #8b5cf6 100%); padding: 12px 20px; border-radius: 12px; margin-bottom: 20px;">
                                <span style="font-size: 24px; font-weight: 700; color: white;">Flow-Like</span>
                            </div>
                            <h1 style="margin: 0; font-size: 28px; font-weight: 700; color: #ffffff; line-height: 1.3;">
                                Your Request Has Been Received! 🎉
                            </h1>
                        </td>
                    </tr>

                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 40px;">
                            <p style="margin: 0 0 24px; font-size: 16px; line-height: 1.6; color: #a3a3a3;">
                                Hi there,
                            </p>
                            <p style="margin: 0 0 24px; font-size: 16px; line-height: 1.6; color: #a3a3a3;">
                                Thank you for submitting your <strong style="color: #ffffff;">24-Hour Solution</strong> request for <strong style="color: #ffffff;">{company_name}</strong>. We're excited to help bring your automation vision to life!
                            </p>

                            <!-- Request Details Card -->
                            <div style="background: #1a1a1a; border: 1px solid #333333; border-radius: 12px; padding: 24px; margin-bottom: 32px;">
                                <h2 style="margin: 0 0 16px; font-size: 14px; font-weight: 600; color: #737373; text-transform: uppercase; letter-spacing: 0.5px;">
                                    Request Details
                                </h2>
                                <div style="display: flex; align-items: center; margin-bottom: 12px;">
                                    <span style="color: #a3a3a3; font-size: 14px;">Plan:</span>
                                    <span style="color: #ffffff; font-size: 14px; font-weight: 600; margin-left: 8px;">{tier_display}</span>
                                    {priority_badge}
                                </div>
                                <div style="margin-bottom: 12px;">
                                    <span style="color: #a3a3a3; font-size: 14px;">Tracking Token:</span>
                                    <code style="color: #3b82f6; font-size: 13px; background: #0a0a0a; padding: 4px 8px; border-radius: 4px; margin-left: 8px; font-family: 'SF Mono', Monaco, monospace;">{tracking_token}</code>
                                </div>
                            </div>

                            <!-- CTA Button -->
                            <div style="text-align: center; margin-bottom: 32px;">
                                <a href="{tracking_url}" style="display: inline-block; background: linear-gradient(135deg, #3b82f6 0%, #8b5cf6 100%); color: white; text-decoration: none; font-size: 16px; font-weight: 600; padding: 16px 32px; border-radius: 12px; box-shadow: 0 4px 14px 0 rgba(59, 130, 246, 0.4);">
                                    Track Your Request →
                                </a>
                            </div>

                            <!-- What's Next -->
                            <div style="background: linear-gradient(135deg, rgba(59, 130, 246, 0.1) 0%, rgba(139, 92, 246, 0.1) 100%); border: 1px solid rgba(59, 130, 246, 0.2); border-radius: 12px; padding: 24px; margin-bottom: 24px;">
                                <h2 style="margin: 0 0 16px; font-size: 18px; font-weight: 600; color: #ffffff;">
                                    What Happens Next?
                                </h2>
                                <ol style="margin: 0; padding-left: 20px; color: #a3a3a3; font-size: 14px; line-height: 1.8;">
                                    <li style="margin-bottom: 8px;"><strong style="color: #ffffff;">Review</strong> — Our team will review your request within the next few hours</li>
                                    <li style="margin-bottom: 8px;"><strong style="color: #ffffff;">Onboarding Call</strong> — We'll schedule a quick call to clarify any details</li>
                                    <li style="margin-bottom: 8px;"><strong style="color: #ffffff;">Development</strong> — Your solution will be built within 24 hours of onboarding</li>
                                    <li><strong style="color: #ffffff;">Delivery</strong> — You'll receive your complete, working solution</li>
                                </ol>
                            </div>

                            <p style="margin: 0; font-size: 14px; line-height: 1.6; color: #737373;">
                                You can track the status of your request at any time using the button above or by visiting:
                                <br>
                                <a href="{tracking_url}" style="color: #3b82f6; text-decoration: none; word-break: break-all;">{tracking_url}</a>
                            </p>
                        </td>
                    </tr>

                    <!-- Footer -->
                    <tr>
                        <td style="padding: 24px 40px; background: #0a0a0a; border-top: 1px solid #262626; text-align: center;">
                            <p style="margin: 0 0 8px; font-size: 14px; color: #737373;">
                                Questions? Reply to this email or contact us at
                                <a href="mailto:help@great-co.de" style="color: #3b82f6; text-decoration: none;">help@great-co.de</a>
                            </p>
                            <p style="margin: 0; font-size: 12px; color: #525252;">
                                © {year} Flow-Like. All rights reserved.
                            </p>
                        </td>
                    </tr>
                </table>
            </td>
        </tr>
    </table>
</body>
</html>"##,
        company_name = company_name,
        tier_display = tier_display,
        priority_badge = priority_badge,
        tracking_token = tracking_token,
        tracking_url = tracking_url,
        year = chrono::Utc::now().format("%Y")
    );

    let text = format!(
        r#"Your 24-Hour Solution Request Has Been Received!

Hi there,

Thank you for submitting your 24-Hour Solution request for {company_name}. We're excited to help bring your automation vision to life!

REQUEST DETAILS
---------------
Plan: {tier_display}{priority_text}
Tracking Token: {tracking_token}

Track your request: {tracking_url}

WHAT HAPPENS NEXT?
------------------
1. Review — Our team will review your request within the next few hours
2. Onboarding Call — We'll schedule a quick call to clarify any details
3. Development — Your solution will be built within 24 hours of onboarding
4. Delivery — You'll receive your complete, working solution

Questions? Reply to this email or contact us at help@great-co.de

© {year} Flow-Like. All rights reserved.
"#,
        company_name = company_name,
        tier_display = tier_display,
        priority_text = if is_priority { " (Priority)" } else { "" },
        tracking_token = tracking_token,
        tracking_url = tracking_url,
        year = chrono::Utc::now().format("%Y")
    );

    (html, text)
}

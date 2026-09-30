// Sign-in and verification email. Table layout with inline styles so it
// renders the same in Gmail, Outlook, Apple Mail and Mail.ru; the plain-text
// part stays for clients that do not show HTML. Only the six digits vary, so
// nothing user-controlled is ever interpolated into the markup.

const COPY = {
  ru: {
    signin: {
      subject: "Код входа BetterFy ID",
      title: "Вход в BetterFy ID",
      lead: "Введи этот код в BetterFy, чтобы войти в аккаунт.",
    },
    link: {
      subject: "Подтверди почту для BetterFy",
      title: "Подключение почты",
      lead: "Введи этот код в BetterFy, чтобы привязать почту к аккаунту.",
    },
    register: {
      subject: "Подтверди почту для BetterFy ID",
      title: "Создание BetterFy ID",
      lead: "Введи этот код в BetterFy, чтобы подтвердить почту и создать аккаунт.",
    },
    expires: "Код действует 10 минут и работает один раз.",
    ignore: "Если ты не запрашивал код, просто проигнорируй это письмо — без кода войти в аккаунт нельзя.",
    footer: "BetterFy · твоя Dota, только лучше",
  },
  en: {
    signin: {
      subject: "Your BetterFy ID sign-in code",
      title: "Sign in to BetterFy ID",
      lead: "Enter this code in BetterFy to sign in to your account.",
    },
    link: {
      subject: "Confirm your email for BetterFy",
      title: "Connect your email",
      lead: "Enter this code in BetterFy to link this email to your account.",
    },
    register: {
      subject: "Confirm your email for BetterFy ID",
      title: "Create your BetterFy ID",
      lead: "Enter this code in BetterFy to confirm your email and create your account.",
    },
    expires: "The code is valid for 10 minutes and works once.",
    ignore: "If you did not request it, ignore this email — nobody can sign in without the code.",
    footer: "BetterFy · your Dota, only better",
  },
};

export function emailContent(code, language, purpose) {
  if (!/^\d{6}$/.test(code)) throw new Error("email_code_invalid");
  const lang = language === "ru" ? "ru" : "en";
  const copy = COPY[lang];
  const kind = copy[purpose] ?? copy.signin;
  const spaced = `${code.slice(0, 3)} ${code.slice(3)}`;
  const font = "Manrope,-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Arial,sans-serif";
  const html = `<!doctype html>
<html lang="${lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="color-scheme" content="dark">
<meta name="supported-color-schemes" content="dark">
<title>${kind.subject}</title>
</head>
<body style="margin:0;padding:0;background:#0d0d12;" bgcolor="#0d0d12">
<div style="display:none;max-height:0;overflow:hidden;opacity:0;">${spaced} — ${copy.expires}</div>
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" bgcolor="#0d0d12" style="background:#0d0d12;">
<tr><td align="center" style="padding:40px 16px;">
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="max-width:480px;">
<tr><td style="padding:0 4px 20px;font-family:${font};">
<span style="font-size:26px;font-weight:800;letter-spacing:-0.03em;color:#f7f5fb;">Better</span><span style="font-size:30px;font-weight:700;font-style:italic;color:#ce57f4;">Fy</span>
</td></tr>
<tr><td bgcolor="#19191f" style="background:#19191f;border:1px solid #3b3844;border-radius:18px;padding:0;">
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0">
<tr><td style="padding:0 32px;"><div style="width:72px;height:2px;background:#b85dea;line-height:2px;font-size:0;">&nbsp;</div></td></tr>
<tr><td style="padding:28px 32px 8px;font-family:${font};font-size:22px;font-weight:700;letter-spacing:-0.02em;color:#f7f5fb;">${kind.title}</td></tr>
<tr><td style="padding:0 32px 24px;font-family:${font};font-size:14px;line-height:22px;color:#b4afbd;">${kind.lead}</td></tr>
<tr><td align="center" style="padding:0 32px;">
<table role="presentation" cellpadding="0" cellspacing="0" border="0" width="100%">
<tr><td align="center" bgcolor="#22222b" style="background:#22222b;border:1px solid #694875;border-radius:14px;padding:22px 12px;font-family:'SF Mono',Menlo,Consolas,'Courier New',monospace;font-size:36px;font-weight:700;letter-spacing:0.18em;color:#f7f5fb;">${spaced}</td></tr>
</table>
</td></tr>
<tr><td style="padding:20px 32px 6px;font-family:${font};font-size:13px;line-height:20px;color:#e4b1f8;">${copy.expires}</td></tr>
<tr><td style="padding:0 32px 30px;font-family:${font};font-size:12px;line-height:19px;color:#827e8d;">${copy.ignore}</td></tr>
</table>
</td></tr>
<tr><td align="center" style="padding:22px 8px 0;font-family:${font};font-size:11px;color:#6f6b7a;">${copy.footer}</td></tr>
</table>
</td></tr>
</table>
</body>
</html>`;
  const text = `${kind.title}\n\n${kind.lead}\n\n${spaced}\n\n${copy.expires}\n${copy.ignore}\n\n${copy.footer}`;
  return { subject: kind.subject, html, text };
}

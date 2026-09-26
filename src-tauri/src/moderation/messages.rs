//! Direct messages Relay sends to members, in the interface language.

use crate::config::HoneypotAction;

fn language(code: &str) -> usize {
    match code {
        "fr" => 1,
        "es" => 2,
        "de" => 3,
        "ru" => 4,
        "zh" => 5,
        "ko" => 6,
        "ja" => 7,
        "id" => 8,
        _ => 0,
    }
}

const TRAP_INTRO: [&str; 9] = [
    "A message from your Discord account was posted in a protected channel used to spot compromised accounts. Your account may have been hijacked, for example by token-stealing malware or a malicious app.",
    "Un message de votre compte Discord a été publié dans un salon protégé qui sert à repérer les comptes piratés. Votre compte a peut-être été détourné, par exemple par un logiciel qui vole les jetons de connexion ou par une application malveillante.",
    "Se publicó un mensaje de tu cuenta de Discord en un canal protegido que sirve para detectar cuentas comprometidas. Es posible que tu cuenta haya sido secuestrada, por ejemplo por un programa que roba tokens o por una aplicación maliciosa.",
    "Von deinem Discord-Konto wurde eine Nachricht in einem geschützten Kanal gepostet, der kompromittierte Konten erkennt. Dein Konto wurde möglicherweise übernommen, etwa durch Schadsoftware, die Tokens stiehlt, oder durch eine bösartige App.",
    "С вашего аккаунта Discord было отправлено сообщение в защищённый канал, который выявляет взломанные аккаунты. Возможно, ваш аккаунт захвачен, например вредоносной программой, крадущей токены, или вредоносным приложением.",
    "你的 Discord 账号在一个用于识别被盗账号的受保护频道中发送了消息。你的账号可能已被盗用，例如被窃取令牌的恶意软件或恶意应用控制。",
    "보호된 채널에 회원님의 Discord 계정으로 메시지가 게시되었습니다. 이 채널은 탈취된 계정을 찾아내는 용도입니다. 토큰을 훔치는 악성 프로그램이나 악성 앱으로 계정이 탈취되었을 수 있습니다.",
    "あなたの Discord アカウントから、乗っ取られたアカウントを見つけるための保護チャンネルにメッセージが投稿されました。トークンを盗むマルウェアや悪意のあるアプリによって、アカウントが乗っ取られている可能性があります。",
    "Sebuah pesan dari akun Discord kamu diposting di kanal terlindungi yang dipakai untuk mendeteksi akun yang dibobol. Akunmu mungkin sudah diambil alih, misalnya oleh malware pencuri token atau aplikasi berbahaya.",
];

const TRAP_KICK: [&str; 9] = [
    "As a precaution, you have been removed from the server. Change your Discord password, enable two-factor authentication and review Authorized Apps before rejoining.",
    "Par précaution, vous avez été expulsé du serveur. Changez votre mot de passe Discord, activez la double authentification et vérifiez les applications autorisées avant de revenir.",
    "Por precaución, se te ha expulsado del servidor. Cambia tu contraseña de Discord, activa la autenticación en dos pasos y revisa las aplicaciones autorizadas antes de volver.",
    "Vorsorglich wurdest du vom Server entfernt. Ändere dein Discord-Passwort, aktiviere die Zwei-Faktor-Authentifizierung und prüfe die autorisierten Apps, bevor du wieder beitrittst.",
    "В качестве меры предосторожности вас удалили с сервера. Смените пароль Discord, включите двухфакторную аутентификацию и проверьте авторизованные приложения, прежде чем вернуться.",
    "为安全起见，你已被移出服务器。请在重新加入前修改 Discord 密码、开启双重验证并检查已授权的应用。",
    "예방 조치로 서버에서 추방되었습니다. 다시 참여하기 전에 Discord 비밀번호를 바꾸고 2단계 인증을 켠 뒤 승인된 앱을 확인하세요.",
    "念のため、サーバーからキックしました。再参加する前に Discord のパスワードを変更し、二要素認証を有効にして、認証済みアプリを確認してください。",
    "Sebagai tindakan pencegahan, kamu dikeluarkan dari server. Ganti kata sandi Discord, aktifkan autentikasi dua faktor, dan periksa Aplikasi Resmi sebelum bergabung kembali.",
];

const TRAP_BAN: [&str; 9] = [
    "As a precaution, you have been banned from the server. Change your Discord password, enable two-factor authentication and review Authorized Apps, then contact the server moderators.",
    "Par précaution, vous avez été banni du serveur. Changez votre mot de passe Discord, activez la double authentification et vérifiez les applications autorisées, puis contactez les modérateurs du serveur.",
    "Por precaución, se te ha baneado del servidor. Cambia tu contraseña de Discord, activa la autenticación en dos pasos, revisa las aplicaciones autorizadas y luego contacta con los moderadores.",
    "Vorsorglich wurdest du vom Server gebannt. Ändere dein Discord-Passwort, aktiviere die Zwei-Faktor-Authentifizierung, prüfe die autorisierten Apps und wende dich dann an die Moderatoren.",
    "В качестве меры предосторожности вас заблокировали на сервере. Смените пароль Discord, включите двухфакторную аутентификацию, проверьте авторизованные приложения и свяжитесь с модераторами.",
    "为安全起见，你已被服务器封禁。请修改 Discord 密码、开启双重验证并检查已授权的应用，然后联系服务器管理员。",
    "예방 조치로 서버에서 차단되었습니다. Discord 비밀번호를 바꾸고 2단계 인증을 켜고 승인된 앱을 확인한 뒤 서버 관리자에게 연락하세요.",
    "念のため、サーバーから BAN しました。Discord のパスワードを変更し、二要素認証を有効にして認証済みアプリを確認したうえで、モデレーターに連絡してください。",
    "Sebagai tindakan pencegahan, kamu diblokir dari server. Ganti kata sandi Discord, aktifkan autentikasi dua faktor, periksa Aplikasi Resmi, lalu hubungi moderator server.",
];

const TRAP_TIMEOUT: [&str; 9] = [
    "As a precaution, you have been muted on the server for a while. Change your Discord password, enable two-factor authentication and review Authorized Apps.",
    "Par précaution, vous avez été exclu temporairement du serveur. Changez votre mot de passe Discord, activez la double authentification et vérifiez les applications autorisées.",
    "Por precaución, se te ha aislado temporalmente en el servidor. Cambia tu contraseña de Discord, activa la autenticación en dos pasos y revisa las aplicaciones autorizadas.",
    "Vorsorglich wurdest du auf dem Server vorübergehend stummgeschaltet. Ändere dein Discord-Passwort, aktiviere die Zwei-Faktor-Authentifizierung und prüfe die autorisierten Apps.",
    "В качестве меры предосторожности вас временно ограничили на сервере. Смените пароль Discord, включите двухфакторную аутентификацию и проверьте авторизованные приложения.",
    "为安全起见，你已在服务器中被暂时禁言。请修改 Discord 密码、开启双重验证并检查已授权的应用。",
    "예방 조치로 서버에서 일시적으로 타임아웃되었습니다. Discord 비밀번호를 바꾸고 2단계 인증을 켠 뒤 승인된 앱을 확인하세요.",
    "念のため、サーバーで一時的にタイムアウトしました。Discord のパスワードを変更し、二要素認証を有効にして、認証済みアプリを確認してください。",
    "Sebagai tindakan pencegahan, kamu dibisukan sementara di server. Ganti kata sandi Discord, aktifkan autentikasi dua faktor, dan periksa Aplikasi Resmi.",
];

const BLOCK_WARNING: [&str; 9] = [
    "Your last message was not shown on stream because it broke the server's rules. Repeated attempts can lead to a timeout.",
    "Votre dernier message n'a pas été affiché sur le stream, car il ne respecte pas les règles du serveur. En cas de répétition, vous pouvez être exclu temporairement.",
    "Tu último mensaje no se mostró en el directo porque no respeta las normas del servidor. Si se repite, puedes recibir un aislamiento temporal.",
    "Deine letzte Nachricht wurde nicht im Stream gezeigt, weil sie gegen die Serverregeln verstößt. Bei Wiederholung kannst du vorübergehend stummgeschaltet werden.",
    "Ваше последнее сообщение не показано на стриме, потому что нарушает правила сервера. При повторении вы можете получить временное ограничение.",
    "你的上一条消息违反了服务器规则，因此没有在直播中显示。如果再次发生，你可能会被暂时禁言。",
    "마지막 메시지가 서버 규칙을 어겨 방송에 표시되지 않았습니다. 반복되면 일시적으로 타임아웃될 수 있습니다.",
    "直前のメッセージはサーバーのルールに反するため、配信に表示されませんでした。繰り返すとタイムアウトになることがあります。",
    "Pesan terakhirmu tidak ditampilkan di stream karena melanggar aturan server. Jika terulang, kamu bisa dibisukan sementara.",
];

pub fn honeypot_notice(action: HoneypotAction, code: &str) -> String {
    let index = language(code);
    let consequence = match action {
        HoneypotAction::Kick => TRAP_KICK[index],
        HoneypotAction::Ban => TRAP_BAN[index],
        HoneypotAction::Timeout => TRAP_TIMEOUT[index],
    };
    format!("{} {consequence}", TRAP_INTRO[index])
}

pub fn block_warning(code: &str) -> &'static str {
    BLOCK_WARNING[language(code)]
}

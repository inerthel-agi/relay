# Changelog

All notable user-facing changes to Relay are documented in this file.
Toutes les évolutions notables de Relay sont documentées dans ce fichier.

Release 1.3.0 and later include every Relay interface language. Earlier releases remain English and French, with English as the fallback in the app.
À partir de la 1.3.0, chaque langue d’interface Relay est incluse. Les versions antérieures restent en anglais et en français, avec l’anglais comme repli dans l’application.

## Versioning policy / Politique de version

- A major Relay update increments the middle number: `1.0.0` → `1.1.0`.
- A minor update, bug fix, or simple addition increments the patch number: `1.0.0` → `1.0.1`.
- Changes remain under `Unreleased` until the matching GitHub release is published.

## [Unreleased]

## [1.3.6] - 2026-09-08

### English

- Separated module cards and controls with consistent spacing, and moved media channel routing to the Media page.

- Reactions now play on Windows through an invisible audio receiver as well as OBS, using the reaction volume and working with the panel hidden.

- Fixed local reaction previews with Web Audio decoding and immediate audio activation; failed playback on one output no longer stops the others.

- Removed the Windows reaction widget; reactions use the OBS source.

- Grouped reaction settings into collapsible sections while keeping sounds and playback controls accessible.

- Replaced the heavy native scrollbars with slim rounded handles, transparent tracks, and accent feedback on hover.

- Widened the expanded navigation sidebar so longer module names remain readable.

- Reaction names now stay in the control panel. OBS and Windows outputs display only the optional visual; sound-only reactions remain invisible.

- Fixed excerpt preview playback with direct Web Audio decoding and immediate audio activation on click, preserving stop and cancellation behavior.

- Fixed reaction audio trimming when the desktop app inherits an outdated PATH by detecting installed WinGet FFmpeg; removed the widget move arrow and kept dragging on its content.

- Removed Windows speech synthesis and voice controls. Message, emoji and sticker notifications, optional sounds and channel cleanup remain available; existing voice settings migrate to visual-only mode.
- Added named output presets, persistent anchors/margins and local portrait, landscape, GIF, video, audio, notification and sticker previews/tests.
- Added a waiting queue with guarded removal, searchable/filterable history and output diagnostics.
- Refactored localization, preset storage, output layout, audio styles, HTTP handlers, Discord music and state caches/playback; separated tests across 19 Rust modules.
- Fixed embedded audio artwork in Now Playing and history with CSP-compatible image loading and bounded shared caching.
- Increased audio artwork to 112 px and reduced the audio card right margin to 4 px.
- Slightly enlarged audio artwork and increased text notification width, height, avatar and text size while retaining right alignment.
- Aligned fitted media and audio to the right edge with a 12 px margin; reduced the notification presence indicator, including its border.
- Removed unintended padding from audio metadata and shifted media and audio farther right within the available space.
- Reduced audio and notification card sizes and nudged fitted media right without changing the YouTube layout.
- Enlarged stickers, compacted sticker-only notifications, matched notification styling to audio cards, and attached author credits below fitted media or inside the audio card while preserving aspect ratios.
- Settings saves now preserve drafts on other pages and edits made while a save is pending. Automatic word filters save only the filter list; queued form saves use the latest saved configuration.
- Reduced introductory headings, removed empty disclosure columns, kept connection status visible in narrow windows, and distinguished unsaved, saved, and error feedback. Failed URL copies now offer a retry.
- Made the protected music welcome message optional, including when automatic cleanup is enabled. An empty field protects no welcome message; a supplied message is never deleted. Routing saves no longer recheck an unchanged music welcome message.
- Added message pinning so the displayed message stays visible while media and music continue; queued messages resume after removal.
- Added a persistent local media library for importing, renaming, replaying and deleting copied images, GIFs and videos.
- Added music request limits and duplicate protection, with controls to move or remove pending tracks.
- Added sounds and reactions, disabled by default, with local tests, OBS and Windows output, named Discord channel and role selectors, cooldowns, and an optional protected instructions message.
- Added a sound excerpt chooser for sounds longer than 30 seconds, with preview, a 30-second limit and preservation of the original file.
- Added bounded FIFO reaction playback with up to 25 waiting requests, localized queue feedback, queue-full handling, and reaction-first behavior for the global skip shortcut; stopping clears all queued reactions.

### Français

- Espacement des blocs et commandes harmonisé ; routage du salon média déplacé dans l’onglet Médias.

- Les réactions sont audibles sous Windows via un lecteur invisible ainsi que dans OBS, au volume de la réaction, même avec le panneau masqué.

- Correction des tests sonores locaux avec décodage Web Audio et activation immédiate ; un échec de lecture sur une sortie n’arrête plus les autres.

- Suppression du widget Windows des réactions ; diffusion via la source OBS.

- Regroupement des réglages des réactions en sections repliables, avec les sons et les commandes de lecture toujours accessibles.

- Remplacement des barres de défilement natives imposantes par des poignées fines et arrondies, des rails transparents et un accent au survol.

- Élargissement de la barre de navigation déployée afin de mieux afficher les noms de modules longs.

- Le nom des réactions reste dans le panneau. OBS et les widgets affichent uniquement le visuel facultatif ; les réactions sonores seules restent invisibles.

- Correction de l’écoute des extraits avec décodage direct par Web Audio et activation audio dès le clic, en conservant l’arrêt et l’annulation.

- Correction de la découpe audio quand l’application hérite d’un PATH ancien grâce à la détection de FFmpeg installé par WinGet ; suppression de la flèche du widget, déplacement conservé sur son contenu.

- Suppression de la synthèse vocale Windows et des réglages de voix. Notifications de messages, emojis et stickers, sons facultatifs et nettoyage des salons conservés ; les anciens réglages vocaux passent en mode visuel.
- Ajout des préréglages de sortie nommés, des ancrages/marges persistants et des aperçus/tests locaux portrait, paysage, GIF, vidéo, audio, notification et sticker.
- Ajout d’une file d’attente avec retrait protégé, d’un historique recherchable et filtrable et de diagnostics des sorties.
- Refactoring des traductions, préréglages, placements, styles audio, routes HTTP, musique Discord et caches/lecture ; séparation des tests de 19 modules Rust.
- Correction des pochettes audio dans Now Playing et l’historique avec un chargement compatible avec la politique de sécurité et un cache partagé borné.
- Pochette audio agrandie à 112 px et marge droite de la carte audio réduite à 4 px.
- Pochette audio légèrement agrandie et notifications texte élargies et rehaussées, avec avatar et texte plus grands, tout en conservant l’alignement à droite.
- Médias et audio alignés au bord droit avec une marge de 12 px ; indicateur de statut des notifications réduit, bordure comprise.
- Suppression des espacements excessifs dans les métadonnées audio et décalage supplémentaire des médias et de l’audio vers la droite, dans l’espace disponible.
- Cartes audio et notifications réduites ; médias légèrement décalés à droite, sans modifier le format YouTube.
- Stickers agrandis, notifications avec sticker seul plus compactes, présentation des notifications harmonisée avec les cartes audio et auteur placé sous le média ajusté ou dans la carte audio, sans déformer les proportions.
- Les enregistrements conservent les brouillons des autres pages et les saisies effectuées pendant une sauvegarde. Le filtre de mots automatique enregistre uniquement sa liste ; les sauvegardes successives utilisent la configuration enregistrée la plus récente.
- Titres introductifs réduits, colonnes vides des sections repliables supprimées, état de connexion visible dans les fenêtres étroites et retours distincts pour les modifications, les sauvegardes et les erreurs. Une copie d’URL échouée permet de réessayer.
- Message d’accueil protégé du salon musique rendu facultatif, y compris lorsque le nettoyage automatique est activé. Un champ vide ne protège aucun message d’accueil ; un message indiqué n’est jamais supprimé. Les enregistrements du routage ne revérifient plus un message d’accueil musique inchangé.
- Ajout de l’épinglage des messages : le message affiché reste visible pendant que les médias et la musique continuent ; la file reprend après son retrait.
- Ajout d’une bibliothèque locale persistante pour importer, renommer, rediffuser et supprimer des copies d’images, de GIF et de vidéos.
- Ajout de limites pour les demandes musicales et d’une protection contre les doublons, avec des boutons pour monter, descendre ou retirer les titres en attente.
- Ajout de sons et réactions désactivés par défaut, avec tests locaux, sorties OBS et Windows, sélecteurs nommés de salons et rôles Discord, délais et message d’instructions protégé facultatif.
- Ajout d’un sélecteur d’extrait pour les sons de plus de 30 secondes, avec aperçu, limite à 30 secondes et conservation du fichier original.
- Ajout d’une file FIFO bornée de 25 réactions en attente, avec position affichée dans le panneau et Discord ; le raccourci global traite d’abord les réactions et l’arrêt vide la file.

### Español

- Espaciado uniforme entre bloques y controles; configuración del canal multimedia trasladada a Medios.

- Las reacciones se oyen en Windows mediante un receptor invisible y en OBS, al volumen de la reacción, incluso con el panel oculto.

- Corregidas las pruebas locales con Web Audio y activación inmediata; un fallo de reproducción en una salida ya no detiene las demás.

- Eliminado el widget de reacciones de Windows; se utiliza la fuente OBS.

- Ajustes de reacciones agrupados en secciones plegables, con sonidos y controles de reproducción accesibles.

- Se sustituyeron las barras de desplazamiento nativas por controles finos y redondeados, pistas transparentes y un acento al pasar el cursor.

- Se amplió la barra de navegación desplegada para mostrar mejor los nombres largos de los módulos.

- Los nombres de las reacciones quedan en el panel. OBS y los widgets muestran solo el visual opcional; las reacciones de solo sonido permanecen invisibles.

- Corregida la escucha de extractos mediante decodificación directa con Web Audio y activación al hacer clic, manteniendo la parada y cancelación.

- Corregido el recorte de audio al detectar FFmpeg instalado por WinGet cuando PATH está desactualizado; eliminada la flecha del widget, cuyo contenido sigue pudiéndose arrastrar.

- Eliminadas la síntesis de voz de Windows y sus controles. Se conservan las notificaciones de mensajes, emojis y stickers, sonidos opcionales y limpieza de canales; los ajustes anteriores pasan al modo visual.
- Añadidos ajustes de salida guardados, anclajes/márgenes persistentes y vistas previas/pruebas locales de imágenes, GIF, vídeo, audio, notificaciones y stickers.
- Añadidas cola con retirada protegida, búsqueda y filtros del historial y diagnósticos de salida.
- Separados traducciones, ajustes, diseño, estilos de audio, rutas HTTP, música Discord, caché y reproducción, además de las pruebas de 19 módulos Rust.
- Corregidas las carátulas de audio en Now Playing y el historial mediante carga compatible con CSP y caché compartida limitada.
- Carátula de audio ampliada a 112 px y margen derecho reducido a 4 px.
- Carátula de audio algo más grande y notificaciones de texto más anchas y altas, con avatar y texto ampliados, manteniendo la alineación derecha.
- Medios y audio alineados al borde derecho con 12 px de margen; indicador de estado reducido, incluido el borde.
- Eliminado el espaciado excesivo del audio y desplazados los medios más a la derecha dentro del espacio disponible.
- Tarjetas de audio y notificaciones más pequeñas; medios desplazados ligeramente a la derecha sin cambiar YouTube.
- Stickers más grandes, notificaciones de solo sticker más compactas, estilo unificado con las tarjetas de audio y autor situado bajo el medio o dentro de la tarjeta de audio, conservando las proporciones.
- El guardado conserva los borradores de otras páginas y los cambios realizados mientras se guarda. El filtro automático guarda solo su lista; los guardados en cola utilizan la última configuración guardada.
- Títulos más compactos, secciones plegables sin columnas vacías, estado de conexión visible en ventanas estrechas e indicadores distintos de cambios, guardado y error. Las copias de URL fallidas permiten reintentar.
- El mensaje de bienvenida protegido del canal de música ahora es opcional, incluso con la limpieza automática activada. Un campo vacío no protege ninguna bienvenida; un mensaje indicado nunca se elimina. Los guardados del enrutamiento ya no vuelven a verificar una bienvenida de música sin cambios.
- Añadido el fijado de mensajes: el mensaje mostrado permanece visible mientras continúan los medios y la música; la cola se reanuda al retirarlo.
- Añadida una biblioteca local persistente para importar, renombrar, volver a emitir y eliminar copias de imágenes, GIF y vídeos.
- Añadidos límites para las solicitudes musicales y protección contra duplicados, con controles para subir, bajar o quitar pistas pendientes.
- Añadidos sonidos y reacciones desactivados de forma predeterminada, con pruebas locales, salidas OBS y Windows, selectores con nombres para canales y roles de Discord, enfriamientos y un mensaje de instrucciones protegido opcional.
- Añadido un selector de fragmentos para sonidos de más de 30 segundos, con vista previa, límite de 30 segundos y conservación del archivo original.
- Añadida una cola FIFO limitada a 25 reacciones en espera, con posición mostrada en el panel y Discord; el atajo global trata primero las reacciones y detenerlas vacía la cola.

### Deutsch

- Abstände zwischen Modulen und Bedienelementen vereinheitlicht; Medienkanal-Routing zur Medienseite verschoben.

- Reaktionen spielen unter Windows über einen unsichtbaren Audioempfänger und in OBS, mit der Reaktionslautstärke und auch bei ausgeblendetem Panel.

- Lokale Reaktionstests nutzen Web Audio und sofortige Aktivierung; Wiedergabefehler einer Ausgabe stoppen die anderen nicht mehr.

- Windows-Reaktionswidget entfernt; Reaktionen verwenden die OBS-Quelle.

- Reaktionseinstellungen in einklappbare Abschnitte gegliedert; Sounds und Wiedergabesteuerung bleiben zugänglich.

- Die auffälligen nativen Bildlaufleisten wurden durch schmale, abgerundete Griffe mit transparenten Spuren und Akzent beim Überfahren ersetzt.

- Die ausgeklappte Navigationsleiste wurde verbreitert, damit längere Modulnamen besser lesbar bleiben.

- Reaktionsnamen bleiben im Bedienfeld. OBS und Widgets zeigen nur das optionale Bild; reine Audioreaktionen bleiben unsichtbar.

- Ausschnittvorschau durch direkte Web-Audio-Dekodierung und sofortige Audioaktivierung beim Klicken korrigiert; Stoppen und Abbrechen bleiben erhalten.

- Audiozuschnitt bei veraltetem PATH durch Erkennung der WinGet-FFmpeg-Installation korrigiert; Widget-Pfeil entfernt, Verschieben am Inhalt bleibt möglich.

- Windows-Sprachausgabe und Sprachsteuerung entfernt. Nachrichten-, Emoji- und Sticker-Benachrichtigungen, optionale Töne und Kanalbereinigung bleiben erhalten; bisherige Spracheinstellungen werden auf visuell umgestellt.
- Benannte Ausgabevorlagen, gespeicherte Anker/Abstände und lokale Vorschauen/Tests für Bilder, GIF, Video, Audio, Benachrichtigungen und Sticker hinzugefügt.
- Warteschlange mit geschütztem Entfernen, Verlaufssuche/-filter und Ausgabediagnose hinzugefügt.
- Übersetzungen, Vorlagen, Layout, Audiostile, HTTP-Routen, Discord-Musik und Zustandscaches/Wiedergabe modularisiert; Tests von 19 Rust-Modulen getrennt.
- Audiocover in Now Playing und Verlauf durch CSP-kompatibles Laden und begrenzten gemeinsamen Cache korrigiert.
- Audiocover auf 112 px vergrößert und rechter Kartenabstand auf 4 px reduziert.
- Audiocover leicht vergrößert; Textbenachrichtigungen breiter und höher mit größerem Avatar und Text, weiterhin rechts ausgerichtet.
- Medien und Audio mit 12 px Abstand rechts ausgerichtet; Statuspunkt einschließlich Rand verkleinert.
- Übermäßige Abstände in Audioangaben entfernt und Medien sowie Audio im verfügbaren Platz weiter nach rechts verschoben.
- Audio- und Benachrichtigungskarten verkleinert; Medien leicht nach rechts verschoben, YouTube unverändert.
- Größere Sticker, kompaktere reine Sticker-Benachrichtigungen, einheitliche Gestaltung mit Audiokarten und Autorenangaben direkt unter Medien oder innerhalb der Audiokarte bei unveränderten Seitenverhältnissen.
- Beim Speichern bleiben Entwürfe anderer Seiten und während des Speicherns eingegebene Änderungen erhalten. Der automatische Wortfilter speichert nur seine Liste; aufeinanderfolgende Speichervorgänge verwenden die zuletzt gespeicherte Konfiguration.
- Kompaktere Überschriften, keine leeren Spalten bei eingeklappten Bereichen, sichtbarer Verbindungsstatus in schmalen Fenstern und getrennte Hinweise für Änderungen, Erfolg und Fehler. Fehlgeschlagene URL-Kopien können wiederholt werden.
- Geschützte Willkommensnachrichten im Musikkanal sind jetzt optional, auch bei aktivierter automatischer Bereinigung. Ein leeres Feld schützt keine Willkommensnachricht; eine angegebene Nachricht wird nie gelöscht. Beim Speichern der Zuordnung wird eine unveränderte Musik-Begrüßung nicht erneut geprüft.
- Nachrichten können jetzt angeheftet werden: Die angezeigte Nachricht bleibt sichtbar, während Medien und Musik weiterlaufen; nach dem Entfernen wird die Warteschlange fortgesetzt.
- Eine dauerhafte lokale Medienbibliothek zum Importieren, Umbenennen, erneuten Ausgeben und Löschen kopierter Bilder, GIFs und Videos wurde hinzugefügt.
- Limits für Musikanfragen und Schutz vor Duplikaten wurden ergänzt; wartende Titel können nach oben, nach unten oder entfernt werden.
- Töne und Reaktionen sind jetzt standardmäßig deaktiviert und bieten lokale Tests, OBS- und Windows-Ausgabe, benannte Discord-Kanal- und Rollenauswahl, Abklingzeiten und eine optionale geschützte Anleitungsnachricht.
- Eine Auswahl für Ausschnitte aus Sounds über 30 Sekunden wurde ergänzt, mit Vorschau, 30-Sekunden-Limit und unveränderter Originaldatei.
- Begrenzte FIFO-Warteschlange für Reaktionen mit bis zu 25 wartenden Anfragen und lokalisierter Positionsanzeige in Panel und Discord hinzugefügt; der globale Shortcut überspringt zuerst Reaktionen und Stoppen leert die Warteschlange.

### Русский

- Выровнены отступы между блоками и элементами управления; настройка медиаканала перенесена на вкладку «Медиа».

- Реакции воспроизводятся в Windows невидимым аудиоплеером и в OBS с заданной громкостью, даже при скрытой панели.

- Локальные тесты используют Web Audio с немедленной активацией; ошибка одного выхода больше не останавливает остальные.

- Виджет реакций Windows удалён; реакции используют источник OBS.

- Настройки реакций сгруппированы в сворачиваемые разделы; звуки и управление воспроизведением остаются доступны.

- Громоздкие системные полосы прокрутки заменены тонкими закруглёнными ползунками с прозрачной дорожкой и акцентом при наведении.

- Расширена открытая панель навигации, чтобы длинные названия модулей оставались читаемыми.

- Названия реакций остаются в панели. OBS и виджеты показывают только выбранное изображение; звуковые реакции без него невидимы.

- Исправлено прослушивание фрагментов через прямое декодирование Web Audio и активацию звука при нажатии; остановка и отмена сохранены.

- Исправлена обрезка звука при устаревшем PATH через обнаружение FFmpeg из WinGet; стрелка виджета удалена, перемещение за содержимое сохранено.

- Удалены синтез речи Windows и настройки голоса. Сохранены уведомления сообщений, эмодзи и стикеров, необязательные звуки и очистка каналов; старые настройки переведены в визуальный режим.
- Добавлены именованные наборы вывода, привязки/отступы и локальные предпросмотры/тесты изображений, GIF, видео, аудио, уведомлений и стикеров.
- Добавлены очередь с защищённым удалением, поиск и фильтры истории и диагностика выходов.
- Разделены переводы, наборы, размещение, стили аудио, HTTP, музыка Discord и кэш/воспроизведение; тесты 19 модулей Rust вынесены отдельно.
- Исправлены обложки аудио в Now Playing и истории: загрузка совместима с CSP, общий кэш ограничен.
- Обложка аудио увеличена до 112 пикселей, правый отступ карточки уменьшен до 4 пикселей.
- Немного увеличена обложка аудио; текстовые уведомления стали шире и выше, с более крупными аватаром и текстом, сохраняя выравнивание справа.
- Медиа и аудио выровнены по правому краю с отступом 12 пикселей; индикатор статуса уменьшен вместе с рамкой.
- Убраны лишние отступы в аудиокарточке, медиа и аудио сдвинуты правее в пределах доступного места.
- Уменьшены карточки аудио и уведомлений; медиа немного сдвинуты вправо, формат YouTube сохранён.
- Увеличены стикеры, уплотнены уведомления только со стикером, оформление согласовано с аудиокарточками. Автор расположен под медиа или внутри аудиокарточки с сохранением пропорций.
- Сохранение настроек сохраняет черновики других страниц и изменения, введённые во время сохранения. Автоматический фильтр слов сохраняет только свой список; очередь сохранений использует последнюю сохранённую конфигурацию.
- Уменьшены вводные заголовки, убраны пустые столбцы сворачиваемых разделов, статус соединения виден в узких окнах. Изменения, сохранение и ошибки имеют разные индикаторы; копирование URL можно повторить после ошибки.
- Защищённое приветственное сообщение музыкального канала теперь необязательно, в том числе при включённой автоматической очистке. Пустое поле ничего не защищает; указанное сообщение никогда не удаляется. При сохранении маршрутизации неизменённое музыкальное приветствие больше не проверяется повторно.
- Добавлено закрепление сообщений: отображаемое сообщение остаётся на экране, пока медиа и музыка продолжают работу; после снятия закрепления очередь возобновляется.
- Добавлена постоянная локальная медиатека для импорта, переименования, повторного показа и удаления копий изображений, GIF и видео.
- Добавлены ограничения музыкальных запросов и защита от дубликатов, а также управление порядком и удалением ожидающих треков.
- Добавлены звуки и реакции, отключённые по умолчанию, с локальным тестированием, выводом в OBS и Windows, именованными селекторами каналов и ролей Discord, задержками и необязательным защищённым сообщением с инструкциями.
- Добавлен выбор фрагмента для звуков длительностью более 30 секунд с предпрослушиванием, ограничением в 30 секунд и сохранением исходного файла.
- Добавлена ограниченная FIFO-очередь до 25 ожидающих реакций с локализованной позицией в панели и Discord; глобальное сочетание сначала пропускает реакции, а остановка очищает очередь.

### 简体中文

- 统一模块和控件间距，并将媒体频道路由移至媒体页面。

- 反应同时通过 Windows 隐藏音频播放器和 OBS 播放，使用反应音量，面板隐藏时仍可播放。

- 本地反应测试使用 Web Audio 解码并立即激活音频；单个输出播放失败不再停止其他输出。

- 移除 Windows 反应小组件；反应通过 OBS 源播放。

- 反应设置分组为可折叠区域，声音和播放控件仍可直接访问。

- 将厚重的原生滚动条替换为纤细圆角滑块、透明轨道和悬停强调效果。

- 加宽展开的导航侧栏，使较长的模块名称更易阅读。

- 反应名称仅显示在控制面板中。OBS 和小组件只显示可选图像，纯声音反应不显示任何内容。

- 使用 Web Audio 直接解码并在点击时立即激活音频，修复片段预览播放，保留停止和取消操作。

- 通过检测 WinGet 安装的 FFmpeg，修复旧 PATH 导致的音频剪辑失败；移除小组件箭头，仍可拖动内容移动。

- 移除Windows语音合成和语音设置。保留消息、表情、贴纸通知、可选提示音和频道清理；旧语音设置迁移为纯视觉模式。
- 新增命名输出预设、持久化锚点/边距，以及图片、GIF、视频、音频、通知和贴纸的本地预览与测试。
- 新增安全移除等待项目的队列、历史搜索/筛选以及输出诊断。
- 拆分翻译、预设、布局、音频样式、HTTP、Discord音乐和缓存/播放模块，并分离19个Rust模块的测试。
- 修复Now Playing和历史记录中的音频封面，使用兼容CSP的加载方式和有容量限制的共享缓存。
- 音频封面增大至112像素，音频卡片右边距缩小至4像素。
- 略微放大音频封面，增加文字通知的宽高、头像及字号，保持右对齐。
- 媒体和音频右对齐并保留12像素边距，缩小状态指示点及其边框。
- 移除音频信息的多余间距，并在可用空间内将媒体和音频进一步右移。
- 缩小音频和通知卡片，媒体略向右移，保持YouTube布局不变。
- 放大贴纸，精简单贴纸通知，使通知与音频卡片风格一致，并将作者放在媒体下方或音频卡片内，同时保持媒体比例。
- 保存设置时保留其他页面的草稿以及保存期间输入的修改。自动词语过滤器仅保存词语列表；排队的保存操作使用最新已保存配置。
- 缩小介绍标题，移除折叠区域的空白列，在窄窗口中保留连接状态，并区分未保存、已保存和错误状态。URL复制失败后可以重试。
- 受保护的音乐频道欢迎消息现为可选项，即使启用自动清理也可以留空。留空时不保护任何欢迎消息；填写的消息永不删除。保存输入路由时不再重复验证未更改的音乐欢迎消息。
- 新增消息固定功能：媒体和音乐继续运行时，当前消息会保持显示；取消固定后队列会继续。
- 新增持久本地媒体库，可导入、重命名、重新播放和删除图片、GIF 与视频副本。
- 新增音乐请求数量限制和重复保护，并可调整或移除等待中的曲目。
- 新增默认关闭的声音与反应功能，支持本地测试、OBS 和 Windows 输出、带名称的 Discord 频道和角色选择、冷却时间及可选的受保护说明消息。
- 新增超过 30 秒声音的片段选择器，支持预览，限制为 30 秒且保留原始文件。
- 新增最多等待 25 个反应的有界 FIFO 队列，在控制面板和 Discord 中显示本地化位置；全局快捷键优先跳过反应，停止操作会清空队列。

### 한국어

- 모듈과 컨트롤 간격을 정리하고 미디어 채널 라우팅을 미디어 페이지로 옮겼습니다.

- 반응은 Windows의 보이지 않는 오디오 수신기와 OBS에서 설정된 볼륨으로 재생되며 패널을 숨겨도 작동합니다.

- 로컬 반응 테스트에 Web Audio 디코딩과 즉시 활성화를 적용했습니다. 한 출력의 재생 실패가 다른 출력을 중지하지 않습니다.

- Windows 반응 위젯을 제거하고 OBS 소스로 반응을 재생합니다.

- 반응 설정을 접을 수 있는 섹션으로 묶고 사운드와 재생 제어는 계속 표시합니다.

- 두꺼운 기본 스크롤바를 투명한 트랙, 얇고 둥근 핸들, 마우스 오버 강조 효과로 교체했습니다.

- 긴 모듈 이름을 더 잘 표시하도록 펼친 탐색 사이드바의 너비를 늘렸습니다.

- 반응 이름은 제어 패널에만 표시됩니다. OBS와 위젯은 선택한 이미지만 표시하며 소리만 있는 반응은 보이지 않습니다.

- Web Audio 직접 디코딩과 클릭 시 즉시 오디오 활성화로 구간 미리 듣기를 수정했으며, 정지와 취소 동작을 유지합니다.

- WinGet으로 설치된 FFmpeg를 찾아 오래된 PATH로 인한 오디오 자르기 오류를 수정했습니다. 위젯 화살표를 제거했으며 콘텐츠를 드래그해 이동할 수 있습니다.

- Windows 음성 합성과 음성 설정을 제거했습니다. 메시지·이모지·스티커 알림, 선택적 알림음과 채널 정리는 유지되며 기존 음성 설정은 시각 알림으로 전환됩니다.
- 이름 있는 출력 프리셋, 저장되는 기준 위치/여백, 이미지·GIF·동영상·오디오·알림·스티커의 로컬 미리 보기와 테스트를 추가했습니다.
- 안전한 항목 제거를 지원하는 대기열, 기록 검색/필터와 출력 진단을 추가했습니다.
- 번역, 프리셋, 배치, 오디오 스타일, HTTP, Discord 음악, 캐시/재생을 분리하고 19개 Rust 모듈의 테스트를 별도 파일로 옮겼습니다.
- CSP 호환 이미지 로딩과 제한된 공유 캐시로 Now Playing 및 기록의 오디오 표지를 수정했습니다.
- 오디오 표지를 112px로 확대하고 카드 오른쪽 여백을 4px로 줄였습니다.
- 오디오 표지를 조금 키우고 텍스트 알림의 너비와 높이, 아바타와 글자를 확대하면서 오른쪽 정렬을 유지했습니다.
- 미디어와 오디오를 오른쪽에 12px 여백으로 정렬하고 상태 표시점을 테두리와 함께 줄였습니다.
- 오디오 정보의 불필요한 여백을 제거하고 미디어와 오디오를 가용 공간 안에서 더 오른쪽으로 이동했습니다.
- 오디오와 알림 카드를 줄이고 미디어를 오른쪽으로 조금 이동했습니다. YouTube 형식은 유지합니다.
- 스티커를 확대하고 스티커 전용 알림을 간결하게 정리했습니다. 알림과 오디오 카드의 스타일을 통일하고 작성자를 미디어 아래 또는 오디오 카드 안에 배치하며 화면 비율을 유지합니다.
- 설정 저장 시 다른 페이지의 초안과 저장 중 입력한 변경 사항을 유지합니다. 자동 단어 필터는 목록만 저장하며, 대기 중인 저장 작업은 최신 저장 설정을 사용합니다.
- 소개 제목을 줄이고 접이식 영역의 빈 열을 제거했습니다. 좁은 창에서도 연결 상태를 표시하며 미저장, 저장 완료, 오류를 구분합니다. URL 복사 실패 후 다시 시도할 수 있습니다.
- 음악 채널의 보호된 환영 메시지를 자동 정리를 켠 경우에도 선택 사항으로 변경했습니다. 비워 두면 어떤 환영 메시지도 보호하지 않으며, 입력한 메시지는 삭제하지 않습니다. 라우팅을 저장할 때 변경되지 않은 음악 환영 메시지를 다시 확인하지 않습니다.
- 메시지 고정을 추가했습니다. 미디어와 음악이 계속 실행되는 동안 현재 메시지를 화면에 유지하며, 고정을 해제하면 대기열이 다시 진행됩니다.
- 이미지, GIF, 동영상 복사본을 가져오고 이름을 바꾸고 다시 방송하거나 삭제할 수 있는 영구 로컬 미디어 라이브러리를 추가했습니다.
- 음악 요청 수 제한과 중복 보호를 추가하고 대기 중인 트랙을 위아래로 이동하거나 제거할 수 있게 했습니다.
- 기본적으로 꺼져 있는 소리 및 반응 기능을 추가했습니다. 로컬 테스트, OBS 및 Windows 출력, 이름이 표시되는 Discord 채널·역할 선택, 대기 시간과 선택적 보호 안내 메시지를 지원합니다.
- 30초가 넘는 사운드에서 구간을 선택하는 기능을 추가했습니다. 미리 듣기와 30초 제한을 지원하며 원본 파일은 보존됩니다.
- 최대 25개의 반응을 기다리게 하는 제한된 FIFO 대기열과 패널·Discord의 현지화된 위치 표시를 추가했습니다. 전역 단축키는 반응을 먼저 건너뛰며 중지하면 대기열을 비웁니다.

### 日本語

- モジュールと操作部分の間隔を整え、メディアチャンネル設定をメディアページへ移動しました。

- リアクションは非表示の Windows 音声プレーヤーと OBS の両方で設定音量で再生され、パネルを隠しても動作します。

- ローカルテストに Web Audio デコードと即時有効化を適用し、1つの出力の再生失敗が他の出力を停止しないよう修正しました。

- Windows リアクションウィジェットを削除し、OBS ソースで再生します。

- リアクション設定を折りたたみ可能なセクションに整理し、サウンドと再生操作は引き続き直接利用できます。

- 太い標準スクロールバーを、透明なトラックと細い丸型ハンドル、ホバー時のアクセント表示に置き換えました。

- 長いモジュール名を読みやすくするため、展開時のナビゲーションサイドバーを広げました。

- リアクション名は操作パネルにのみ表示。OBSとウィジェットは任意の画像だけを表示し、音声のみのリアクションは非表示になります。

- Web Audioによる直接デコードとクリック時の音声有効化で抜粋のプレビュー再生を修正。停止とキャンセルの動作は維持しました。

- WinGetでインストールされたFFmpegを検出し、古いPATHによる音声切り抜きの失敗を修正。ウィジェットの矢印を削除し、内容のドラッグによる移動は維持しました。

- Windows音声合成と音声設定を削除しました。メッセージ・絵文字・ステッカー通知、任意の通知音、チャンネル整理は維持し、従来の音声設定は表示のみへ移行します。
- 名前付き出力プリセット、保存可能な基準位置・余白、画像・GIF・動画・音声・通知・ステッカーのローカルプレビューとテストを追加しました。
- 安全に取り除ける待機キュー、履歴の検索・絞り込み、出力診断を追加しました。
- 翻訳、プリセット、配置、音声スタイル、HTTP、Discord音楽、キャッシュ・再生を分離し、19個のRustモジュールのテストを別ファイルに移動しました。
- CSPに対応した画像読み込みと上限付き共有キャッシュで、Now Playingと履歴の音声アートワークを修正しました。
- 音声アートワークを112pxに拡大し、カードの右余白を4pxに縮小しました。
- 音声のアートワークを少し拡大し、文字通知の幅・高さ・アバター・文字を大きくしました。右寄せは維持します。
- メディアと音声を右端から12pxで配置し、通知の状態表示を枠線込みで小さくしました。
- 音声情報の余分な余白を除去し、利用可能な範囲でメディアと音声をさらに右へ移動しました。
- 音声・通知カードを小さくし、メディアを少し右へ移動しました。YouTubeの形式は維持します。
- ステッカーを拡大し、ステッカーのみの通知をコンパクトにしました。通知と音声カードのデザインを統一し、縦横比を保ちながら投稿者をメディアの下または音声カード内に配置します.
- 設定保存時に他のページの下書きと保存中の入力を保持します。自動単語フィルターは単語一覧のみを保存し、待機中の保存処理は最新の保存済み設定を使用します。
- 導入見出しを小さくし、折りたたみ領域の空の列をなくしました。狭いウィンドウでも接続状態を表示し、未保存・保存済み・エラーを区別します。URLのコピー失敗後に再試行できます。
- 音楽チャンネルの保護案内メッセージを、音楽の自動整理を有効にした場合も任意にしました。空欄では案内メッセージを保護せず、指定したメッセージは削除されません。ルーティング保存時に変更されていない音楽案内メッセージを再確認しません。
- メッセージ固定を追加しました。メディアや音楽の再生中も表示中のメッセージを保持し、固定を解除すると待機キューが再開します。
- 画像、GIF、動画のコピーをインポート、名前変更、再配信、削除できる永続ローカルメディアライブラリを追加しました。
- 音楽リクエストの上限と重複保護を追加し、待機中のトラックを上下に移動または削除できるようにしました。
- デフォルトで無効なサウンドとリアクションを追加しました。ローカルテスト、OBS と Windows 出力、名前付きの Discord チャンネル・ロール選択、クールダウン、任意の保護案内メッセージに対応します。
- 30 秒を超えるサウンドから、プレビュー付きで 30 秒以内の範囲を選べる機能を追加しました。元のファイルは保持されます。
- 最大25件のリアクションを待機できる上限付きFIFOキューと、パネル・Discordでの位置表示を追加しました。グローバルショートカットはリアクションを優先してスキップし、停止するとキューを消去します。

### Bahasa Indonesia

- Merapikan jarak antarblok dan kontrol serta memindahkan pengaturan kanal media ke halaman Media.

- Reaksi diputar melalui penerima audio Windows yang tidak terlihat serta OBS, memakai volume reaksi dan tetap berjalan saat panel disembunyikan.

- Pengujian lokal menggunakan dekode Web Audio dan aktivasi langsung; kegagalan satu keluaran tidak lagi menghentikan keluaran lainnya.

- Widget reaksi Windows dihapus; reaksi menggunakan sumber OBS.

- Pengaturan reaksi dikelompokkan dalam bagian yang dapat dilipat; suara dan kontrol pemutaran tetap dapat diakses.

- Mengganti scrollbar bawaan yang tebal dengan pegangan tipis membulat, jalur transparan, dan aksen saat diarahkan.

- Memperlebar sidebar navigasi saat dibuka agar nama modul yang panjang tetap mudah dibaca.

- Nama reaksi hanya ditampilkan di panel kontrol. OBS dan widget hanya menampilkan visual opsional; reaksi suara saja tetap tidak terlihat.

- Memperbaiki pratinjau cuplikan melalui dekode Web Audio langsung dan aktivasi audio saat diklik, dengan fungsi berhenti dan batal tetap tersedia.

- Memperbaiki pemotongan audio saat PATH usang dengan mendeteksi FFmpeg dari WinGet; menghapus panah widget dan tetap mengizinkan pemindahan dengan menyeret kontennya.

- Menghapus sintesis suara Windows dan pengaturan suara. Notifikasi pesan, emoji, stiker, bunyi opsional dan pembersihan kanal tetap tersedia; pengaturan lama beralih ke mode visual.
- Menambahkan preset keluaran bernama, jangkar/margin tersimpan, serta pratinjau dan uji lokal gambar, GIF, video, audio, notifikasi dan stiker.
- Menambahkan antrean dengan penghapusan terlindungi, pencarian/filter riwayat dan diagnosis keluaran.
- Memisahkan terjemahan, preset, tata letak, gaya audio, HTTP, musik Discord dan cache/pemutaran; pengujian 19 modul Rust dipisahkan.
- Memperbaiki sampul audio di Now Playing dan riwayat dengan pemuatan sesuai CSP serta cache bersama terbatas.
- Sampul audio diperbesar menjadi 112 px dan jarak kanan kartu diperkecil menjadi 4 px.
- Sampul audio sedikit diperbesar; notifikasi teks diperlebar dan ditinggikan dengan avatar serta teks lebih besar, tetap rata kanan.
- Media dan audio diratakan ke kanan dengan jarak 12 px; indikator status diperkecil termasuk tepinya.
- Menghapus jarak berlebih pada informasi audio dan menggeser media serta audio lebih ke kanan dalam ruang tersedia.
- Kartu audio dan notifikasi diperkecil, media digeser sedikit ke kanan, format YouTube tetap.
- Stiker diperbesar, notifikasi khusus stiker dibuat ringkas, gaya notifikasi disamakan dengan kartu audio, dan pengirim ditempatkan di bawah media atau di dalam kartu audio tanpa mengubah rasio aspek.
- Penyimpanan pengaturan mempertahankan draf halaman lain dan perubahan yang diketik saat penyimpanan berlangsung. Filter kata otomatis hanya menyimpan daftarnya; antrean penyimpanan menggunakan konfigurasi tersimpan terbaru.
- Judul pengantar diperkecil, kolom kosong pada bagian lipat dihapus, status koneksi tetap terlihat di jendela sempit, dan perubahan, penyimpanan, serta kesalahan ditandai berbeda. Penyalinan URL yang gagal dapat dicoba kembali.
- Pesan sambutan terlindungi di channel musik kini opsional, termasuk saat pembersihan otomatis diaktifkan. Kolom kosong tidak melindungi pesan sambutan apa pun; pesan yang ditentukan tidak pernah dihapus. Penyimpanan perutean tidak lagi memverifikasi ulang pesan sambutan musik yang tidak berubah.
- Menambahkan penyematan pesan: pesan yang sedang ditampilkan tetap terlihat saat media dan musik terus berjalan; antrean dilanjutkan setelah penyematan dilepas.
- Menambahkan pustaka media lokal permanen untuk mengimpor, mengganti nama, menyiarkan ulang, dan menghapus salinan gambar, GIF, dan video.
- Menambahkan batas permintaan musik dan perlindungan duplikat, serta kontrol untuk menaikkan, menurunkan, atau menghapus trek yang menunggu.
- Menambahkan suara dan reaksi yang nonaktif secara default, dengan pengujian lokal, keluaran OBS dan Windows, pemilih kanal dan peran Discord berdasarkan nama, jeda penggunaan, serta pesan instruksi terlindungi yang opsional.
- Menambahkan pemilih cuplikan untuk suara lebih dari 30 detik, dengan pratinjau, batas 30 detik, dan file asli tetap dipertahankan.
- Menambahkan antrean FIFO terbatas untuk hingga 25 reaksi yang menunggu dengan posisi terjemahan di panel dan Discord; pintasan global melewati reaksi terlebih dahulu dan tombol berhenti mengosongkan antrean.

## [1.3.3] - 2026-09-04

### English

- Fixed scroll containment in short windows so expanded settings and their save buttons remain reachable; the sidebar scrolls independently.
- Updated release, changelog, and updater links to the current stealthsrc/relay repository.
- Added separate opt-in 24-hour cleanup for the media and TTS channels under Input routing, with optional protected welcome messages and five-minute history checks.
- Notifications are wider and display up to six text lines, wrapping long text even without spaces.
- The OBS music credit shows the track title and Discord requester instead of the YouTube channel name.
- Added automatic music channel cleanup with a verified, protected welcome message, 120-second result expiration, and a separate preview and confirmation for historical messages.
- Added a requester-only Loop button alongside Skip. Repeats keep the selected range, reuse the Discord card, and yield to queued work. Media volume help now explicitly includes YouTube.
- Automatically delete the triggering honeypot message after the kick or ban attempt, including when the action fails. Report deletion failures in the bot status.
- Added collapsible sections in Overview, Music, and Overlay, including size and crop controls.
- Removed the unused notification YouTube player and its styles; test-only Rust helpers are excluded from release builds.

### Français

- Correction du défilement dans les fenêtres basses pour garder les paramètres dépliés et leurs boutons d’enregistrement accessibles ; la barre latérale défile indépendamment.
- Mise à jour des liens de release, du changelog et de l’outil de mise à jour vers le dépôt actuel stealthsrc/relay.
- Ajout du nettoyage optionnel après 24 heures pour les salons médias et TTS dans le routage des entrées, avec messages d’accueil protégés facultatifs et vérification de l’historique toutes les cinq minutes.
- Les notifications sont plus larges et affichent jusqu’à six lignes de texte, avec retour à la ligne même sans espaces.
- Le cartouche musical OBS affiche le titre du morceau et le demandeur Discord à la place du nom de la chaîne YouTube.
- Ajout du nettoyage automatique du salon musique avec message d’accueil vérifié et protégé, expiration des résultats après 120 secondes et aperçu avec confirmation pour l’historique.
- Ajout du bouton Loop, réservé au demandeur, à côté de Skip. La boucle conserve l’extrait choisi et la carte Discord, et laisse passer les demandes en attente. L’aide du volume mentionne désormais YouTube.
- Supprime automatiquement le message déclencheur du honeypot après la tentative de kick ou de ban, même si elle échoue. Signale les échecs de suppression dans l’état du bot.
- Ajout de sections repliables dans Overview, Music et Overlay, y compris les réglages de taille et de recadrage.
- Suppression de l’ancien lecteur YouTube inutilisé des notifications et de ses styles ; les helpers Rust réservés aux tests sont exclus des builds de production.

### Español

- Corregido el desplazamiento en ventanas bajas para acceder a los ajustes abiertos y sus botones de guardado; la barra lateral se desplaza de forma independiente.
- Actualizados los enlaces de versiones, historial de cambios y actualización al repositorio actual stealthsrc/relay.
- Añadida limpieza opcional tras 24 horas para los canales de medios y TTS en el enrutamiento de entradas, con bienvenida protegida opcional y revisión del historial cada cinco minutos.
- Las notificaciones son más anchas y muestran hasta seis líneas, incluso con texto largo sin espacios.
- El crédito musical de OBS muestra el título y el solicitante de Discord en lugar del nombre del canal de YouTube.
- Añadida limpieza automática del canal de música con bienvenida verificada y protegida, resultados que caducan a los 120 segundos y vista previa con confirmación para el historial.
- Añadido Loop junto a Skip, solo para quien solicita la canción. Conserva el fragmento y la tarjeta de Discord y cede el turno a la cola. La ayuda del volumen incluye YouTube.
- Elimina automáticamente el mensaje que activa la trampa tras intentar expulsar o banear, incluso si la acción falla. Muestra los fallos de eliminación en el estado del bot.
- Añadidas secciones plegables en Overview, Music y Overlay, incluidos los ajustes de tamaño y recorte.
- Eliminados el reproductor YouTube sin uso de las notificaciones y sus estilos; las funciones Rust exclusivas de pruebas se excluyen de producción.

### Deutsch

- Scrollbereiche in niedrigen Fenstern korrigiert, damit geöffnete Einstellungen und Speichern-Schaltflächen erreichbar bleiben; die Seitenleiste scrollt unabhängig.
- Release-, Änderungsprotokoll- und Updater-Links auf das aktuelle Repository stealthsrc/relay umgestellt.
- Optionale 24-Stunden-Bereinigung für Medien- und TTS-Kanäle unter Eingangszuordnung hinzugefügt, mit optional geschützter Begrüßung und Verlaufsprüfung alle fünf Minuten.
- Benachrichtigungen sind breiter und zeigen bis zu sechs Textzeilen, auch bei langen Texten ohne Leerzeichen.
- Die OBS-Musikeinblendung zeigt den Titel und den Discord-Anfragenden statt des YouTube-Kanalnamens.
- Automatische Musikkanalbereinigung mit geprüfter, geschützter Willkommensnachricht, 120 Sekunden gültigen Ergebnissen sowie Vorschau und Bestätigung für den Verlauf hinzugefügt.
- Loop neben Skip ist nur für den Anfragenden verfügbar. Wiederholungen behalten Ausschnitt und Discord-Karte und lassen wartende Einträge vor. Die Lautstärkehilfe nennt jetzt YouTube.
- Löscht die auslösende Honeypot-Nachricht automatisch nach dem Kick- oder Ban-Versuch, auch wenn dieser fehlschlägt. Zeigt Löschfehler im Bot-Status an.
- Einklappbare Bereiche in Overview, Music und Overlay einschließlich Größe und Zuschnitt hinzugefügt.
- Ungenutzten YouTube-Player der Benachrichtigungen und seine Stile entfernt; reine Rust-Testhelfer werden nicht mehr im Release gebaut.

### Русский

- Исправлена прокрутка в невысоких окнах: развёрнутые настройки и кнопки сохранения остаются доступны, боковая панель прокручивается отдельно.
- Ссылки на релизы, журнал изменений и обновления переведены на текущий репозиторий stealthsrc/relay.
- В маршрутизацию входов добавлена отдельная очистка медиаканала и TTS через 24 часа с необязательным защищённым приветствием и проверкой истории каждые пять минут.
- Уведомления стали шире и показывают до шести строк с переносом длинного текста без пробелов.
- Музыкальная подпись OBS показывает название трека и автора запроса в Discord вместо названия канала YouTube.
- Добавлена автоматическая очистка музыкального канала с проверенным защищённым приветствием, удалением результатов через 120 секунд и отдельным предпросмотром с подтверждением очистки истории.
- Рядом со Skip добавлена кнопка Loop для автора запроса. Повтор сохраняет отрывок и карточку Discord, уступая очередь другим запросам. В описании громкости указан YouTube.
- Автоматически удаляет сообщение, активировавшее ловушку, после попытки исключения или блокировки, даже если она не удалась. Ошибки удаления отображаются в статусе бота.
- Добавлены сворачиваемые разделы Overview, Music и Overlay, включая размер и обрезку.
- Удалены неиспользуемый проигрыватель YouTube уведомлений и его стили; тестовые функции Rust исключены из релизных сборок.

### 简体中文

- 修复较矮窗口中的滚动，使展开的设置和保存按钮保持可访问；侧边栏可独立滚动。
- 发布、更新日志和更新程序链接已改为当前的 stealthsrc/relay 仓库。
- 在输入路由中新增媒体和TTS频道的独立24小时清理选项，可保护欢迎消息，每五分钟检查一次历史记录。
- 通知更宽，最多显示六行文字，无空格的长文本也会自动换行。
- OBS 音乐信息显示曲目标题和 Discord 请求者，不再显示 YouTube 频道名称。
- 新增音乐频道自动清理：验证并保护欢迎消息，搜索结果在120秒后过期，历史消息清理需要单独预览和确认。
- 在 Skip 旁新增仅请求者可用的 Loop 按钮。循环保留所选片段和 Discord 卡片，并让等待中的请求先播放。音量说明明确包含 YouTube。
- 尝试踢出或封禁后自动删除触发陷阱的消息，即使操作失败也会尝试删除。删除失败会显示在机器人状态中。
- 在 Overview、Music 和 Overlay 中新增可折叠区域，包括尺寸和裁剪设置。
- 移除了通知中未使用的 YouTube 播放器及其样式；仅供测试的 Rust 辅助函数不再编入发布版本。

### 한국어

- 높이가 작은 창에서도 펼친 설정과 저장 버튼에 접근하도록 스크롤을 수정했습니다. 사이드바는 별도로 스크롤됩니다.
- 릴리스, 변경 기록 및 업데이터 링크를 현재 stealthsrc/relay 저장소로 변경했습니다.
- 입력 라우팅에 미디어 및 TTS 채널의 선택적 24시간 정리를 추가했습니다. 환영 메시지를 보호할 수 있으며 5분마다 기록을 확인합니다.
- 알림 너비를 늘리고 최대 여섯 줄을 표시하며 공백 없는 긴 텍스트도 줄바꿈합니다.
- OBS 음악 정보에 YouTube 채널 이름 대신 곡 제목과 Discord 요청자를 표시합니다.
- 확인된 환영 메시지를 보호하는 음악 채널 자동 정리를 추가했습니다. 검색 결과는 120초 후 만료되며 기록 정리는 미리 보기와 확인이 필요합니다.
- Skip 옆에 요청자 전용 Loop 버튼을 추가했습니다. 선택한 구간과 Discord 카드를 유지하고 대기 중인 요청에 순서를 양보합니다. 볼륨 설명에 YouTube를 명시했습니다.
- 추방 또는 차단 시도 후 실패 여부와 관계없이 함정을 작동시킨 메시지를 자동으로 삭제합니다. 삭제 실패는 봇 상태에 표시됩니다.
- Overview, Music, Overlay에 크기 및 자르기 설정을 포함한 접이식 섹션을 추가했습니다.
- 알림의 사용하지 않는 YouTube 플레이어와 스타일을 제거하고 테스트 전용 Rust 함수를 릴리스 빌드에서 제외했습니다.

### 日本語

- 高さの低いウィンドウでも展開した設定と保存ボタンにアクセスできるようスクロールを修正しました。サイドバーは独立してスクロールします。
- リリース、変更履歴、更新機能のリンクを現在の stealthsrc/relay リポジトリに変更しました。
- 入力ルーティングにメディアとTTSチャンネル個別の24時間整理を追加しました。案内メッセージを任意で保護し、5分ごとに履歴を確認します。
- 通知の幅を広げ、最大6行を表示します。空白のない長い文章も折り返します。
- OBS の音楽表示に YouTube チャンネル名ではなく曲名と Discord のリクエスト者を表示します。
- 確認済みの案内メッセージを保護する音楽チャンネルの自動整理を追加しました。検索結果は120秒で失効し、履歴の整理にはプレビューと確認が必要です。
- Skip の隣にリクエスト者専用の Loop を追加しました。選択区間と Discord カードを保持し、待機中のリクエストに順番を譲ります。音量説明に YouTube を明記しました。
- キックまたは BAN の試行後、処理が失敗してもトラップを作動させたメッセージを自動削除します。削除の失敗は Bot の状態に表示されます。
- Overview、Music、Overlayにサイズとトリミングを含む折りたたみ式セクションを追加しました。
- 通知の未使用YouTubeプレーヤーとスタイルを削除し、テスト専用Rust関数をリリースビルドから除外しました。

### Bahasa Indonesia

- Memperbaiki gulir pada jendela pendek agar pengaturan terbuka dan tombol simpan tetap terjangkau; bilah samping bergulir sendiri.
- Memperbarui tautan rilis, changelog, dan pembaruan ke repositori stealthsrc/relay saat ini.
- Menambahkan pembersihan 24 jam opsional terpisah untuk channel media dan TTS pada perutean input, dengan sambutan terlindungi opsional dan pemeriksaan riwayat setiap lima menit.
- Notifikasi lebih lebar dan menampilkan hingga enam baris, termasuk teks panjang tanpa spasi.
- Kredit musik OBS menampilkan judul lagu dan peminta Discord, bukan nama channel YouTube.
- Menambahkan pembersihan channel musik otomatis dengan pesan sambutan terverifikasi dan dilindungi, hasil kedaluwarsa setelah 120 detik, serta pratinjau dan konfirmasi terpisah untuk riwayat.
- Menambahkan Loop di samping Skip, hanya untuk peminta lagu. Pengulangan mempertahankan cuplikan dan kartu Discord serta memberi giliran antrean. Bantuan volume kini menyebut YouTube.
- Otomatis menghapus pesan pemicu perangkap setelah percobaan kick atau ban, termasuk jika tindakan gagal. Kegagalan penghapusan ditampilkan dalam status bot.
- Menambahkan bagian lipat di Overview, Music, dan Overlay, termasuk ukuran dan pemotongan.
- Menghapus pemutar YouTube notifikasi yang tidak digunakan beserta gayanya; fungsi Rust khusus pengujian tidak disertakan dalam build rilis.

## [1.3.2] - 2026-09-03

### English

#### Changed

- The native window title bar now follows the selected interface theme and its colors instead of switching between fixed light and dark styles.
- The GitHub profile link in the panel now points to the current creator profile.
- Automatic moderation now includes a configurable compromised account trap with a Discord channel selector, Kick or Ban action, and an English security notice for affected users.

#### Fixed

- Fixed inherited application icons lingering on the native title bar after theme changes.
- Restored the Relay logo and `Relay` title in Windows taskbar previews while keeping both hidden from the app's native title bar.

### Français

#### Modifié

- La barre de titre native suit désormais le thème d'interface sélectionné et ses couleurs, au lieu d'alterner entre des styles clair et sombre fixes.
- Le lien de profil GitHub du panneau pointe désormais vers le profil actuel du créateur.
- La modération automatique propose désormais un piège à compte compromis configurable, avec choix du salon Discord, action Exclure ou Bannir et avertissement de sécurité en anglais pour l’utilisateur concerné.

#### Corrigé

- Correction des icônes héritées qui restaient sur la barre de titre native après un changement de thème.
- Restauration du logo Relay et du titre `Relay` dans les aperçus de la barre des tâches Windows, tout en les masquant dans la barre de titre native de l’application.

### Español

#### Cambiado

- La barra de título nativa ahora sigue el tema de interfaz seleccionado y sus colores, en lugar de alternar entre estilos claros y oscuros fijos.
- El enlace de perfil de GitHub del panel apunta ahora al perfil actual del creador.
- La moderación automática incluye ahora una trampa configurable para cuentas comprometidas, con selector de canal de Discord, acción Expulsar o Banear y aviso de seguridad en inglés para el usuario afectado.

#### Corregido

- Corregidos los iconos heredados que permanecían en la barra de título nativa tras cambiar de tema.
- Restaurados el logotipo de Relay y el título `Relay` en las vistas previas de la barra de tareas de Windows, manteniéndolos ocultos en la barra de título nativa de la aplicación.

### Deutsch

#### Geändert

- Die native Titelleiste folgt jetzt dem ausgewählten Oberflächenthema und dessen Farben, statt nur zwischen festen hellen und dunklen Stilen zu wechseln.
- Der GitHub-Profil-Link im Panel zeigt jetzt auf das aktuelle Erstellerprofil.
- Die automatische Moderation enthält jetzt eine konfigurierbare Falle für kompromittierte Konten mit Discord-Kanalauswahl, Kick- oder Ban-Aktion und englischem Sicherheitshinweis für betroffene Benutzer.

#### Behoben

- Ererbte Symbole, die nach einem Themenwechsel auf der nativen Titelleiste verblieben, werden jetzt entfernt.
- Das Relay-Logo und der Titel `Relay` werden wieder in den Vorschauen der Windows-Taskleiste angezeigt, bleiben aber in der nativen Titelleiste der App ausgeblendet.

### Русский

#### Изменено

- Нативная строка заголовка теперь следует выбранной теме интерфейса и её цветам вместо переключения между фиксированными светлым и тёмным стилями.
- Ссылка на профиль GitHub в панели теперь ведёт на актуальный профиль автора.
- В автоматическую модерацию добавлена настраиваемая ловушка для скомпрометированных аккаунтов с выбором канала Discord, действием Kick или Ban и предупреждением на английском языке для затронутого пользователя.

#### Исправлено

- Исправлена проблема с унаследованными значками приложения, остававшимися в нативной строке заголовка после смены темы.
- В предварительных просмотрах панели задач Windows восстановлены логотип Relay и заголовок `Relay`, при этом в нативной строке заголовка приложения они скрыты.

### 简体中文

#### 更改

- 原生窗口标题栏现在跟随所选的界面主题及其颜色，而不再只在固定的浅色和深色样式之间切换。
- 面板中的 GitHub 主页链接现在指向当前的创作者主页。
- 自动审核现已加入可配置的被盗账号陷阱，支持选择 Discord 频道、Kick 或 Ban 操作，并向受影响用户发送英文安全通知。

#### 已修复

- 修复了切换主题后原生标题栏残留继承应用图标的问题。
- Windows 任务栏预览中重新显示 Relay 标志和 `Relay` 标题，同时在应用的原生标题栏中将两者隐藏。

### 한국어

#### 변경

- 네이티브 창 제목 표시줄이 이제 고정된 밝은/어두운 스타일 사이를 전환하는 대신 선택한 인터페이스 테마와 색상을 따릅니다.
- 패널의 GitHub 프로필 링크가 이제 현재 크리에이터 프로필을 가리킵니다.
- 자동 검토에 Discord 채널 선택, Kick 또는 Ban 작업, 영향을 받은 사용자에게 보내는 영어 보안 알림을 갖춘 탈취 계정 함정이 추가되었습니다.

#### 수정됨

- 테마 변경 후 네이티브 제목 표시줄에 남아 있던 상속된 앱 아이콘 문제를 수정했습니다.
- Windows 작업 표시줄 미리 보기에 Relay 로고와 `Relay` 제목을 복원하고 앱의 네이티브 제목 표시줄에서는 둘 다 숨겼습니다.

### 日本語

#### 変更

- ネイティブのタイトルバーが、固定のライト/ダークスタイル間の切り替えではなく、選択したインターフェーステーマとその色に追従するようになりました。
- パネルの GitHub プロフィールリンクを現在のクリエイタープロフィールに更新しました。
- 自動モデレーションに、Discord チャンネル、Kick または Ban の処理、対象ユーザーへの英語のセキュリティ通知を選べる乗っ取りアカウント用トラップを追加しました。

#### 修正

- テーマ変更後にネイティブのタイトルバーに残っていた継承アイコンの問題を修正しました。
- Windows タスクバーのプレビューに Relay ロゴと `Relay` タイトルを復元し、アプリのネイティブタイトルバーでは両方を非表示にしました。

### Bahasa Indonesia

#### Diubah

- Bilah judul jendela native kini mengikuti tema antarmuka yang dipilih beserta warnanya, alih-alih berganti antara gaya terang dan gelap yang tetap.
- Tautan profil GitHub di panel kini menunjuk ke profil pembuat saat ini.
- Moderasi otomatis kini memiliki perangkap akun yang disusupi dengan pilihan channel Discord, tindakan Kick atau Ban, serta pemberitahuan keamanan berbahasa Inggris untuk pengguna terdampak.

#### Diperbaiki

- Memperbaiki ikon aplikasi warisan yang tersisa di bilah judul native setelah perubahan tema.
- Memulihkan logo Relay dan judul `Relay` pada pratinjau bilah tugas Windows sambil menyembunyikan keduanya dari bilah judul native aplikasi.

## [1.3.1] - 2026-08-17

### English

#### Added

- Added `/relay nuke <channel>` for Discord administrators to recreate a text or announcement channel and permanently remove its complete message history. Relay updates its configured channel references automatically; the bot requires **Manage Channels**.

#### Fixed

- Fixed the TTS notification card leaving an unused transparent strip in wide or resized Windows widgets.
- Fixed live media previews rendering transparency as white in the Relay panel. Previews now use a black backdrop without affecting OBS transparency.

#### Security

- Stopped using the vulnerable transitive versions `quick-xml 0.39.4` and `rustls-webpki 0.102.8` by moving file dialogs and Discord TLS to Windows-native backends.

### Français

#### Ajouté

- Ajout de `/relay nuke <channel>` pour les administrateurs Discord afin de recréer un salon textuel ou d'annonces et supprimer définitivement l'intégralité de son historique. Relay met automatiquement à jour ses références de salon ; le bot requiert la permission **Gérer les salons**.

#### Corrigé

- Correction de la carte de notification TTS qui laissait une bande transparente inutilisée dans les widgets Windows larges ou redimensionnés.
- Correction des aperçus média en direct qui affichaient la transparence en blanc dans le panneau Relay. Les aperçus utilisent désormais un fond noir sans modifier la transparence OBS.

#### Sécurité

- Abandon des versions transitives vulnérables `quick-xml 0.39.4` et `rustls-webpki 0.102.8` en migrant les sélecteurs de fichiers et le TLS Discord vers les backends natifs Windows.

### Español

#### Añadido

- Añadido `/relay nuke <channel>` para que los administradores de Discord puedan recrear un canal de texto o anuncios y eliminar permanentemente todo su historial de mensajes. Relay actualiza sus referencias de canal automáticamente; el bot requiere **Gestionar canales**.

#### Corregido

- Corregida la tarjeta de notificación TTS que dejaba una franja transparente sin usar en widgets de Windows anchos o redimensionados.
- Corregidas las vistas previas de medios en directo que mostraban la transparencia en blanco en el panel de Relay. Ahora usan un fondo negro sin afectar la transparencia de OBS.

#### Seguridad

- Se dejó de usar las versiones transitivas vulnerables `quick-xml 0.39.4` y `rustls-webpki 0.102.8` al migrar los selectores de archivos y el TLS de Discord a los backends nativos de Windows.

### Deutsch

#### Hinzugefügt

- `/relay nuke <channel>` für Discord-Administratoren hinzugefügt, um einen Text- oder Ankündigungs-Kanal neu zu erstellen und seinen gesamten Nachrichtenverlauf dauerhaft zu löschen. Relay aktualisiert seine gespeicherten Kanalreferenzen automatisch; der Bot benötigt **Kanäle verwalten**.

#### Behoben

- Die TTS-Benachrichtigungskarte behoben, die in breiten oder in der Größe geänderten Windows-Widgets einen ungenutzten transparenten Streifen ließ.
- Live-Medienvorschauen behoben, die Transparenz im Relay-Panel weiß darstellten. Vorschauen verwenden jetzt einen schwarzen Hintergrund, ohne die OBS-Transparenz zu beeinflussen.

#### Sicherheit

- Die verwundbaren transitiven Versionen `quick-xml 0.39.4` und `rustls-webpki 0.102.8` werden nicht mehr verwendet, da Dateidialoge und Discord-TLS auf native Windows-Backends umgestellt wurden.

### Русский

#### Добавлено

- Добавлена команда `/relay nuke <channel>` для администраторов Discord: она заново создаёт текстовый канал или канал объявлений и навсегда удаляет всю историю сообщений. Relay автоматически обновляет сохранённые ссылки на канал; боту требуется право **Управление каналами**.

#### Исправлено

- Исправлена карточка TTS-уведомления, которая оставляла неиспользуемую прозрачную полосу в широких или изменённых по размеру виджетах Windows.
- Исправлен показ прозрачности белым цветом в живых предпросмотрах медиа на панели Relay. Теперь предпросмотры используют чёрный фон, не влияя на прозрачность OBS.

#### Безопасность

- От применения уязвимых транзитивных версий `quick-xml 0.39.4` и `rustls-webpki 0.102.8` отказано благодаря переходу диалогов выбора файлов и TLS Discord на нативные бэкенды Windows.

### 简体中文

#### 新增

- 新增供 Discord 管理员使用的 `/relay nuke <channel>`：重新创建文本或公告频道，并永久删除其全部消息记录。Relay 会自动更新已配置的频道引用；机器人需要 **管理频道** 权限。

#### 已修复

- 修复了 TTS 通知卡片在宽尺寸或调整大小后的 Windows 小组件中留下未使用透明条的问题。
- 修复了 Relay 面板中的实时媒体预览将透明背景显示为白色的问题。预览现在使用黑色背景，不影响 OBS 的透明度。

#### 安全

- 改用 Windows 原生文件选择器和 Discord TLS 后端，不再使用存在漏洞的间接依赖版本 `quick-xml 0.39.4` 和 `rustls-webpki 0.102.8`。

### 한국어

#### 추가

- Discord 관리자가 텍스트 또는 공지 채널을 다시 만들고 전체 메시지 기록을 영구적으로 삭제할 수 있는 `/relay nuke <channel>`를 추가했습니다. Relay는 설정된 채널 참조를 자동으로 갱신하며, 봇에는 **채널 관리** 권한이 필요합니다.

#### 수정됨

- 넓거나 크기가 조정된 Windows 위젯에서 TTS 알림 카드가 사용되지 않는 투명 영역을 남기던 문제를 수정했습니다.
- Relay 패널의 실시간 미디어 미리보기에서 투명도가 흰색으로 표시되던 문제를 수정했습니다. 이제 미리보기는 OBS 투명도에 영향을 주지 않는 검은색 배경을 사용합니다.

#### 보안

- Windows 기본 파일 대화 상자와 Discord TLS 백엔드로 전환하여 취약한 간접 의존성 버전 `quick-xml 0.39.4` 및 `rustls-webpki 0.102.8`의 사용을 중단했습니다.

### 日本語

#### 追加

- Discord 管理者がテキストチャンネルまたは告知チャンネルを再作成し、すべてのメッセージ履歴を完全に削除できる `/relay nuke <channel>` を追加しました。Relay は設定済みのチャンネル参照を自動更新し、ボットには **チャンネル管理** 権限が必要です。

#### 修正

- 幅が広い、またはサイズ変更された Windows ウィジェットで、TTS 通知カードの周囲に未使用の透明な帯が残る問題を修正しました。
- Relay パネルのライブメディアプレビューで透明部分が白く表示される問題を修正しました。OBS の透明度に影響を与えず、黒い背景で表示されます。

#### セキュリティ

- Windows ネイティブのファイルダイアログと Discord TLS バックエンドへ切り替え、脆弱な間接依存関係バージョン `quick-xml 0.39.4` と `rustls-webpki 0.102.8` を使用しないようにしました。

### Bahasa Indonesia

#### Ditambahkan

- Menambahkan `/relay nuke <channel>` agar administrator Discord dapat membuat ulang channel teks atau pengumuman dan menghapus seluruh riwayat pesannya secara permanen. Relay memperbarui referensi channel yang dikonfigurasi secara otomatis; bot memerlukan izin **Kelola Channel**.

#### Diperbaiki

- Memperbaiki kartu notifikasi TTS yang menyisakan strip transparan tidak terpakai pada widget Windows yang lebar atau telah diubah ukurannya.
- Memperbaiki pratinjau media langsung yang menampilkan transparansi sebagai putih di panel Relay. Pratinjau kini memakai latar belakang hitam tanpa memengaruhi transparansi OBS.

#### Keamanan

- Berhenti memakai versi dependensi transitif rentan `quick-xml 0.39.4` dan `rustls-webpki 0.102.8` dengan beralih ke backend native Windows untuk dialog file dan TLS Discord.

## [1.3.0] - 2026-08-17

### English

#### Added

- Added YouTube music search in a configurable Discord channel, with up to 15 relevant results between one and five minutes long.
- Added 30-second previews, full-track playback, queueing, and a Now Playing card for OBS and the Windows widget.
- Added unified **Relay Visual** and **Relay Audio** OBS Browser Sources for media, stickers, TTS notifications, YouTube playback, Discord audio, and TTS voice. Legacy source URLs remain available during migration.
- Added History downloads, a configurable global media skip shortcut, and an English YouTube API setup guide in the Music panel and project documentation.
- Added the **Gridline** and **Lumen** interface designs, compact and dynamic sidebar layouts, and a collapsible design picker.
- Added CI dependency auditing for the Rust crate so known high-severity issues are checked on every verification run.
- Added an in-app **Changelog** page that shows bundled release notes in the current interface language.

#### Changed

- Media, TTS, and YouTube playback now share output scheduling so competing items do not play over one another.
- YouTube playback now respects widget sound settings and wakes the Windows audio output when needed.
- Media captions now stay compact and anchored to the active media or player card, with independent OBS and Windows widget visibility settings.
- Windows TTS notifications now use a denser toast (400×104 by default) that stays compact instead of stretching across leftover empty space. Existing generated 980×180 and 480×112 sizes migrate automatically; custom sizes are kept.
- Overlay move labels and preview copy now follow every Relay interface language, including Russian, Simplified Chinese, Korean, Japanese, and Indonesian.
- Language, theme, accent, and font-scale choices are now saved with Relay config and restored on launch, including tray-only and `--startup` sessions.

#### Fixed

- Fixed YouTube playback restoration after output wake-up and stop/start transitions.
- Fixed skip shortcut registration failures without discarding the previous shortcut.
- Fixed canceled media downloads leaving History actions in a busy state.
- Fixed GIF downloads accepting responses that were not GIF files.
- Fixed TTS and sticker Browser Sources dropping their queued items after a brief WebSocket drop.
- Fixed TTS and sticker outputs starting before the server granted the shared stage, which could overlap media or music.
- Fixed OBS TTS and sticker sources navigating blindly after a Relay port change. They now probe the new server first, with a timeout, so a failed load does not leave the Browser Source dead.
- Fixed the Windows media widget forgetting its position when hidden or locked immediately after a move.
- Fixed deferred Discord GIF updates ignoring privacy filter exemptions because role information was missing.
- Fixed YouTube track selections being discarded when the jukebox queue was already full.
- Fixed Relay reporting a Windows TTS failure after a successful visual fallback.

#### Security

- YouTube API keys are stored locally in Windows Credential Manager and are not shown again after saving.
- YouTube playback controls are restricted to the user who requested the track or a Discord administrator.
- Panel and overlay Tauri windows now use separate permission sets: overlay widgets can no longer invoke control-panel commands.
- Discord overlay URLs posted by the bot no longer include the local Relay secret. Short pages inject a page-local secret instead of a host-wide cookie.
- Unused privacy-threshold settings that no longer affected filtering were removed from the interface and config schema.

### Français

#### Ajouté

- Ajout de la recherche musicale YouTube dans un salon Discord configurable, avec jusqu’à 15 résultats pertinents d’une à cinq minutes.
- Ajout des extraits de 30 secondes, de la lecture complète, de la file d’attente et d’une carte En cours de lecture pour OBS et le widget Windows.
- Ajout des sources navigateur OBS unifiées **Relay Visual** et **Relay Audio** pour les médias, les stickers, les notifications TTS, YouTube, l’audio Discord et la voix TTS. Les anciennes URL restent disponibles pendant la migration.
- Ajout des téléchargements depuis l’Historique, d’un raccourci global de saut média configurable, et d’un guide YouTube API en anglais dans le panneau Musique et la documentation.
- Ajout des designs d’interface **Gridline** et **Lumen**, des dispositions de barre latérale compacte et dynamique, et d’un sélecteur de design repliable.
- Ajout d’un audit des dépendances Rust en CI afin de signaler les vulnérabilités connues à chaque vérification.
- Ajout d’une page **Changelog** dans l’application, qui affiche les notes de version incluses dans la langue de l’interface.

#### Modifié

- La lecture des médias, du TTS et de YouTube partage désormais un ordonnancement commun pour éviter les chevauchements.
- La lecture YouTube respecte désormais le son du widget et réveille la sortie audio Windows si besoin.
- Les légendes média restent compactes et ancrées à la carte active, avec des réglages de visibilité indépendants pour OBS et le widget Windows.
- Les notifications TTS Windows utilisent désormais un toast plus dense (400×104 par défaut) qui ne s’étire plus dans le vide restant. Les tailles générées 980×180 et 480×112 sont migrées automatiquement ; les tailles personnalisées sont conservées.
- Les libellés de déplacement de l’overlay et les textes d’aperçu suivent désormais toutes les langues de Relay, y compris le russe, le chinois simplifié, le coréen, le japonais et l’indonésien.
- La langue, le thème, l’accent et l’échelle de police sont désormais enregistrés avec la configuration Relay et restaurés au lancement, y compris en mode zone de notification et `--startup`.

#### Corrigé

- Correction de la restauration YouTube après un réveil de sortie et les transitions arrêt/reprise.
- Correction de l’enregistrement du raccourci de saut qui pouvait échouer en oubliant le raccourci précédent.
- Correction des téléchargements média annulés qui laissaient les actions de l’Historique bloquées.
- Correction des téléchargements GIF qui acceptaient des réponses n’étant pas des fichiers GIF.
- Correction des sources TTS et stickers qui perdaient leur file d’attente après une coupure WebSocket brève.
- Correction des sorties TTS et stickers qui démarraient avant l’accord du serveur, ce qui pouvait chevaucher un média ou la musique.
- Correction des sources OBS TTS et stickers qui changeaient de port sans vérifier le nouveau serveur. Elles sondent désormais d’abord la nouvelle instance, avec un délai, afin d’éviter une Browser Source définitivement bloquée.
- Correction du widget média Windows qui oubliait sa position s’il était masqué ou verrouillé juste après un déplacement.
- Correction des GIF Discord différés qui ignoraient les exemptions de filtrage faute de rôles.
- Correction des sélections YouTube perdues lorsque la file du jukebox était déjà pleine.
- Correction du statut Relay qui signalait un échec TTS Windows alors que le repli visuel avait réussi.

#### Sécurité

- Les clés API YouTube sont stockées localement dans le Gestionnaire d’identifiants Windows et ne sont plus réaffichées après l’enregistrement.
- Les contrôles de lecture YouTube sont réservés à l’utilisateur qui a demandé le morceau ou à un administrateur Discord.
- Les fenêtres Tauri du panneau et des overlays utilisent désormais des jeux de permissions séparés : les widgets overlay ne peuvent plus invoquer les commandes du panneau de contrôle.
- Les URL d’overlay publiées par le bot Discord n’incluent plus le secret Relay local. Les pages courtes injectent un secret limité à la page plutôt qu’un cookie pour tout l’hôte.
- Les réglages de seuils de confidentialité inutilisés, qui n’influençaient plus le filtrage, ont été retirés de l’interface et du schéma de configuration.

### Español

#### Añadido

- Añadida la búsqueda musical de YouTube en un canal de Discord configurable, con hasta 15 resultados relevantes de entre uno y cinco minutos.
- Añadidos extractos de 30 segundos, reproducción completa, cola y una tarjeta En reproducción para OBS y el widget de Windows.
- Añadidas las fuentes de navegador OBS unificadas **Relay Visual** y **Relay Audio** para medios, stickers, notificaciones TTS, reproducción de YouTube, audio de Discord y voz TTS. Las URL de fuentes antiguas siguen disponibles durante la migración.
- Añadidas las descargas del Historial, un atajo global configurable para saltar el medio, y una guía de YouTube API en inglés en el panel Música y la documentación.
- Añadidos los diseños de interfaz **Gridline** y **Lumen**, las disposiciones de barra lateral compacta y dinámica, y un selector de diseño plegable.
- Añadida una auditoría de dependencias Rust en CI para detectar problemas conocidos de alta gravedad en cada verificación.
- Añadida una página **Changelog** en la aplicación que muestra las notas de versión incluidas en el idioma de la interfaz.

#### Cambiado

- La reproducción de medios, TTS y YouTube comparte ahora una programación común para que los elementos no se solapen.
- La reproducción de YouTube respeta ahora el sonido del widget y despierta la salida de audio de Windows cuando hace falta.
- Los subtítulos de medios permanecen compactos y anclados a la tarjeta activa, con ajustes de visibilidad independientes para OBS y el widget de Windows.
- Las notificaciones TTS de Windows usan ahora un toast más denso (400×104 por defecto) que permanece compacto en lugar de estirarse por el espacio vacío. Los tamaños generados 980×180 y 480×112 se migran automáticamente; los tamaños personalizados se conservan.
- Las etiquetas de desplazamiento del overlay y los textos de vista previa siguen ahora todos los idiomas de Relay, incluidos el ruso, el chino simplificado, el coreano, el japonés y el indonesio.
- El idioma, el tema, el acento y la escala de fuente se guardan ahora con la configuración de Relay y se restauran al iniciar, incluidas las sesiones de bandeja y `--startup`.

#### Corregido

- Corregida la restauración de YouTube tras un despertar de salida y las transiciones de parada/reanudación.
- Corregido el registro del atajo de salto que podía fallar y olvidar el atajo anterior.
- Corregidas las descargas de medios canceladas que dejaban las acciones del Historial bloqueadas.
- Corregidas las descargas GIF que aceptaban respuestas que no eran archivos GIF.
- Corregidas las fuentes TTS y stickers que perdían su cola tras una breve caída de WebSocket.
- Corregidas las salidas TTS y stickers que arrancaban antes de que el servidor concediera el escenario compartido, lo que podía solaparse con un medio o la música.
- Corregidas las fuentes OBS TTS y stickers que cambiaban de puerto sin comprobar el nuevo servidor. Ahora sondean primero la nueva instancia, con un tiempo de espera, para no dejar la Browser Source bloqueada.
- Corregido el widget de medios de Windows que olvidaba su posición si se ocultaba o bloqueaba justo después de un movimiento.
- Corregidos los GIF de Discord diferidos que ignoraban las exenciones del filtro de privacidad por falta de roles.
- Corregidas las selecciones de YouTube que se descartaban cuando la cola del jukebox ya estaba llena.
- Corregido el estado de Relay que informaba de un fallo TTS de Windows tras un repliegue visual correcto.

#### Seguridad

- Las claves API de YouTube se almacenan localmente en el Administrador de credenciales de Windows y no se vuelven a mostrar tras guardar.
- Los controles de reproducción de YouTube están restringidos a quien pidió la pista o a un administrador de Discord.
- Las ventanas Tauri del panel y de los overlays usan ahora conjuntos de permisos separados: los widgets overlay ya no pueden invocar comandos del panel de control.
- Las URL de overlay publicadas por el bot de Discord ya no incluyen el secreto local de Relay. Las páginas cortas inyectan un secreto limitado a la página en lugar de una cookie para todo el host.
- Los ajustes de umbral de privacidad no usados, que ya no afectaban al filtrado, se retiraron de la interfaz y del esquema de configuración.

### Deutsch

#### Hinzugefügt

- YouTube-Musiksuche in einem konfigurierbaren Discord-Kanal hinzugefügt, mit bis zu 15 relevanten Ergebnissen zwischen einer und fünf Minuten.
- 30-Sekunden-Vorschauen, vollständige Wiedergabe, Warteschlange und eine Now-Playing-Karte für OBS und das Windows-Widget hinzugefügt.
- Vereinheitlichte OBS-Browserquellen **Relay Visual** und **Relay Audio** für Medien, Sticker, TTS-Benachrichtigungen, YouTube-Wiedergabe, Discord-Audio und TTS-Stimme hinzugefügt. Alte Quellen-URLs bleiben während der Migration verfügbar.
- Downloads im Verlauf, eine konfigurierbare globale Tastenkombination zum Überspringen von Medien sowie eine englische YouTube-API-Anleitung im Musikbereich und in der Dokumentation hinzugefügt.
- Die Oberflächendesigns **Gridline** und **Lumen**, kompakte und dynamische Seitenleistenlayouts sowie einen einklappbaren Designwähler hinzugefügt.
- CI-Abhängigkeitsprüfung für die Rust-Crate hinzugefügt, damit bekannte schwerwiegende Probleme bei jeder Prüfung erkannt werden.
- Eine **Changelog**-Seite in der App hinzugefügt, die gebündelte Versionshinweise in der aktuellen Oberflächensprache anzeigt.

#### Geändert

- Medien, TTS und YouTube teilen sich jetzt eine gemeinsame Ausgabeplanung, damit sich Inhalte nicht überschneiden.
- Die YouTube-Wiedergabe berücksichtigt jetzt die Widget-Toneinstellungen und weckt bei Bedarf die Windows-Audioausgabe.
- Medienuntertitel bleiben kompakt und an der aktiven Medien- oder Playerkarte verankert, mit unabhängigen Sichtbarkeitseinstellungen für OBS und das Windows-Widget.
- Windows-TTS-Benachrichtigungen verwenden jetzt einen dichteren Toast (standardmäßig 400×104), der kompakt bleibt, statt in den restlichen Leerraum zu strecken. Die generierten Größen 980×180 und 480×112 werden automatisch migriert; benutzerdefinierte Größen bleiben erhalten.
- Overlay-Verschiebebeschriftungen und Vorschautexte folgen jetzt jeder Relay-Oberflächensprache, einschließlich Russisch, Vereinfachtes Chinesisch, Koreanisch, Japanisch und Indonesisch.
- Sprache, Thema, Akzentfarbe und Textskalierung werden jetzt mit der Relay-Konfiguration gespeichert und beim Start wiederhergestellt, einschließlich Infobereich- und `--startup`-Sitzungen.

#### Behoben

- Wiederherstellung der YouTube-Wiedergabe nach Ausgabe-Aufwecken und Stopp/Start-Übergängen behoben.
- Fehlerhafte Registrierung der Überspringen-Tastenkombination behoben, ohne die vorherige Tastenkombination zu verwerfen.
- Abgebrochene Medien-Downloads behoben, die Verlaufsaktionen im Beschäftigt-Zustand ließen.
- GIF-Downloads behoben, die Antworten akzeptierten, die keine GIF-Dateien waren.
- TTS- und Sticker-Browserquellen behoben, die nach einem kurzen WebSocket-Abbruch ihre Warteschlange verloren.
- TTS- und Sticker-Ausgaben behoben, die starteten, bevor der Server die gemeinsame Bühne gewährte, was Medien oder Musik überlappen konnte.
- OBS-TTS- und Sticker-Quellen behoben, die nach einem Relay-Portwechsel blind navigierten. Sie prüfen jetzt zuerst den neuen Server, mit Timeout, damit eine fehlgeschlagene Ladung die Browserquelle nicht tot hinterlässt.
- Windows-Medienwidget behoben, das seine Position vergaß, wenn es direkt nach einem Verschieben ausgeblendet oder gesperrt wurde.
- Verzögerte Discord-GIF-Updates behoben, die Datenschutzfilter-Ausnahmen ignorierten, weil Rolleninformationen fehlten.
- YouTube-Titelauswahl behoben, die verworfen wurde, wenn die Jukebox-Warteschlange bereits voll war.
- Relay-Status behoben, der einen Windows-TTS-Fehler meldete, obwohl der visuelle Fallback erfolgreich war.

#### Sicherheit

- YouTube-API-Schlüssel werden lokal im Windows-Anmeldeinformations-Manager gespeichert und nach dem Speichern nicht erneut angezeigt.
- YouTube-Wiedergabesteuerung ist auf die Person beschränkt, die den Titel angefordert hat, oder auf einen Discord-Administrator.
- Tauri-Fenster von Panel und Overlays verwenden jetzt getrennte Berechtigungssätze: Overlay-Widgets können keine Steuerpanel-Befehle mehr aufrufen.
- Vom Discord-Bot veröffentlichte Overlay-URLs enthalten nicht mehr das lokale Relay-Geheimnis. Kurze Seiten injizieren ein seitenlokales Geheimnis statt eines hostweiten Cookies.
- Ungenutzte Datenschutz-Schwellenwerte, die das Filtern nicht mehr beeinflussten, wurden aus Oberfläche und Konfigurationsschema entfernt.

### Русский

#### Добавлено

- Добавлен поиск музыки YouTube в настраиваемом канале Discord, до 15 релевантных результатов длительностью от одной до пяти минут.
- Добавлены 30-секундные превью, полное воспроизведение, очередь и карточка «Сейчас играет» для OBS и виджета Windows.
- Добавлены единые источники браузера OBS **Relay Visual** и **Relay Audio** для медиа, стикеров, TTS-уведомлений, YouTube, аудио Discord и голоса TTS. Старые URL источников остаются доступны во время миграции.
- Добавлены загрузки из истории, настраиваемый глобальный ярлык пропуска медиа и руководство YouTube API на английском в разделе «Музыка» и в документации.
- Добавлены дизайны интерфейса **Gridline** и **Lumen**, компактная и динамическая боковые панели, а также сворачиваемый выбор дизайна.
- Добавлена проверка зависимостей Rust в CI, чтобы известные серьёзные уязвимости выявлялись при каждой проверке.
- Добавлена страница **Changelog** в приложении, которая показывает встроенные заметки о версии на языке интерфейса.

#### Изменено

- Воспроизведение медиа, TTS и YouTube теперь использует общее планирование вывода, чтобы элементы не накладывались друг на друга.
- Воспроизведение YouTube теперь учитывает звук виджета и при необходимости будит аудиовыход Windows.
- Подписи к медиа остаются компактными и привязаны к активной карточке, с независимой видимостью для OBS и виджета Windows.
- TTS-уведомления Windows теперь используют более плотный тост (по умолчанию 400×104), который не растягивается на пустое место. Сгенерированные размеры 980×180 и 480×112 переносятся автоматически; пользовательские размеры сохраняются.
- Подписи перемещения оверлея и тексты предпросмотра теперь следуют всем языкам интерфейса Relay, включая русский, упрощённый китайский, корейский, японский и индонезийский.
- Язык, тема, акцент и масштаб шрифта теперь сохраняются в конфигурации Relay и восстанавливаются при запуске, включая режим области уведомлений и `--startup`.

#### Исправлено

- Исправлено восстановление YouTube после пробуждения вывода и переходов остановки/возобновления.
- Исправлена регистрация ярлыка пропуска, которая могла завершаться ошибкой и забывать предыдущий ярлык.
- Исправлены отменённые загрузки медиа, из-за которых действия истории зависали.
- Исправлены загрузки GIF, которые принимали ответы, не являющиеся файлами GIF.
- Исправлены источники TTS и стикеров, которые теряли очередь после короткого обрыва WebSocket.
- Исправлены выводы TTS и стикеров, которые запускались до разрешения сервера, из-за чего могли перекрывать медиа или музыку.
- Исправлены источники OBS TTS и стикеров, которые меняли порт, не проверяя новый сервер. Теперь они сначала опрашивают новый экземпляр с тайм-аутом, чтобы Browser Source не оставался мёртвым.
- Исправлен виджет медиа Windows, который забывал позицию, если его скрывали или блокировали сразу после перемещения.
- Исправлены отложенные GIF Discord, которые игнорировали исключения фильтра конфиденциальности из-за отсутствия ролей.
- Исправлен сброс выбора YouTube, когда очередь jukebox уже была полной.
- Исправлен статус Relay, который сообщал об ошибке TTS Windows после успешного визуального запасного варианта.

#### Безопасность

- Ключи API YouTube хранятся локально в диспетчере учётных данных Windows и больше не показываются после сохранения.
- Управление воспроизведением YouTube доступно только запросившему трек пользователю или администратору Discord.
- Окна Tauri панели и оверлеев теперь используют разные наборы прав: виджеты оверлея больше не могут вызывать команды панели управления.
- URL оверлея, публикуемые Discord-ботом, больше не содержат локальный секрет Relay. Короткие страницы внедряют секрет только для страницы, а не cookie на весь хост.
- Неиспользуемые пороги конфиденциальности, которые больше не влияли на фильтрацию, удалены из интерфейса и схемы конфигурации.

### 简体中文

#### 新增

- 新增可配置 Discord 频道中的 YouTube 音乐搜索，最多 15 条时长一到五分钟的相关结果。
- 新增 30 秒试听、完整播放、队列，以及用于 OBS 和 Windows 小组件的正在播放卡片。
- 新增统一的 OBS 浏览器源 **Relay Visual** 和 **Relay Audio**，覆盖媒体、贴纸、TTS 通知、YouTube 播放、Discord 音频和 TTS 语音。迁移期间仍可使用旧源 URL。
- 新增历史记录下载、可配置的全局媒体跳过快捷键，以及音乐面板和文档中的英文 YouTube API 设置指南。
- 新增 **Gridline** 和 **Lumen** 界面设计、紧凑与动态侧边栏布局，以及可折叠的设计选择器。
- 新增 Rust 依赖的 CI 审计，以便每次检查都能发现已知的高危问题。
- 新增应用内 **Changelog** 页面，按当前界面语言显示内置版本说明。

#### 更改

- 媒体、TTS 和 YouTube 播放现在共享输出调度，避免互相重叠。
- YouTube 播放现在遵循小组件声音设置，并在需要时唤醒 Windows 音频输出。
- 媒体说明保持紧凑并锚定到当前媒体或播放卡片，OBS 与 Windows 小组件的可见性可分别设置。
- Windows TTS 通知现在使用更紧凑的提示条（默认 400×104），不再拉伸填满空白。已生成的 980×180 和 480×112 尺寸会自动迁移；自定义尺寸会保留。
- 叠加层移动标签和预览文案现在跟随 Relay 的所有界面语言，包括俄语、简体中文、韩语、日语和印尼语。
- 语言、主题、强调色和字体缩放现在随 Relay 配置保存，并在启动时恢复，包括托盘会话和 `--startup`。

#### 修复

- 修复输出唤醒以及停止/开始切换后的 YouTube 播放恢复。
- 修复跳过快捷键注册失败时丢掉上一个快捷键的问题。
- 修复取消的媒体下载使历史记录操作一直处于忙碌状态。
- 修复 GIF 下载会接受非 GIF 文件响应的问题。
- 修复 TTS 和贴纸浏览器源在短暂 WebSocket 中断后丢失队列。
- 修复 TTS 和贴纸输出在服务器授予共享舞台之前就开始，可能与媒体或音乐重叠。
- 修复 OBS 的 TTS 和贴纸源在 Relay 端口变更后盲目跳转。现在会先探测新服务器并设超时，避免 Browser Source 彻底失效。
- 修复 Windows 媒体小组件在移动后立即隐藏或锁定时忘记位置。
- 修复延迟的 Discord GIF 更新因缺少角色信息而忽略隐私过滤豁免。
- 修复点唱机队列已满时 YouTube 曲目选择被丢弃。
- 修复视觉回退后 Relay 仍报告 Windows TTS 失败。

#### 安全

- YouTube API 密钥存储在本地 Windows 凭据管理器中，保存后不再显示。
- YouTube 播放控制仅限请求该曲目的用户或 Discord 管理员。
- 面板和叠加层的 Tauri 窗口现在使用不同权限集：叠加层小组件无法再调用控制面板命令。
- Discord 机器人发布的叠加层 URL 不再包含本地 Relay 密钥。短页面注入仅限该页的密钥，而不是整机 cookie。
- 已不再影响过滤的无用隐私阈值设置已从界面和配置架构中移除。

### 한국어

#### 추가

- 설정 가능한 Discord 채널에서 YouTube 음악 검색을 추가했으며, 1~5분 길이의 관련 결과를 최대 15개까지 표시합니다.
- 30초 미리 듣기, 전체 재생, 대기열, OBS 및 Windows 위젯용 지금 재생 중 카드를 추가했습니다.
- 미디어, 스티커, TTS 알림, YouTube 재생, Discord 오디오, TTS 음성을 위한 통합 OBS 브라우저 소스 **Relay Visual** 및 **Relay Audio**를 추가했습니다. 마이그레이션 중에는 기존 소스 URL을 계속 사용할 수 있습니다.
- 기록 다운로드, 설정 가능한 전역 미디어 건너뛰기 단축키, 음악 패널과 문서의 영어 YouTube API 설정 가이드를 추가했습니다.
- **Gridline** 및 **Lumen** 인터페이스 디자인, 축소/동적 사이드바 레이아웃, 접을 수 있는 디자인 선택기를 추가했습니다.
- 검증마다 알려진 고위험 문제를 확인하도록 Rust 크레이트 CI 의존성 감사를 추가했습니다.
- 현재 인터페이스 언어로 포함된 릴리스 노트를 보여주는 앱 내 **Changelog** 페이지를 추가했습니다.

#### 변경

- 미디어, TTS, YouTube 재생이 서로 겹치지 않도록 출력 일정을 공유합니다.
- YouTube 재생이 위젯 소리 설정을 따르며 필요할 때 Windows 오디오 출력을 깨웁니다.
- 미디어 캡션이 활성 미디어 또는 플레이어 카드에 고정된 채 작게 유지되며, OBS와 Windows 위젯 표시 여부를 따로 설정할 수 있습니다.
- Windows TTS 알림이 더 밀도 높은 토스트(기본 400×104)를 사용해 빈 공간을 채우도록 늘어나지 않습니다. 생성된 980×180 및 480×112 크기는 자동 이전되며 사용자 지정 크기는 유지됩니다.
- 오버레이 이동 레이블과 미리 보기 문구가 러시아어, 간체 중국어, 한국어, 일본어, 인도네시아어를 포함한 모든 Relay 인터페이스 언어를 따릅니다.
- 언어, 테마, 강조색, 글꼴 배율이 Relay 구성과 함께 저장되며 트레이 전용 및 `--startup` 세션을 포함해 시작 시 복원됩니다.

#### 수정

- 출력 깨우기 및 중지/시작 전환 후 YouTube 재생 복원을 수정했습니다.
- 이전 단축키를 버리지 않고 건너뛰기 단축키 등록 실패를 수정했습니다.
- 취소된 미디어 다운로드가 기록 작업을 바쁨 상태로 남기던 문제를 수정했습니다.
- GIF가 아닌 응답을 받아들이던 GIF 다운로드를 수정했습니다.
- 짧은 WebSocket 끊김 후 TTS 및 스티커 브라우저 소스가 대기열을 잃던 문제를 수정했습니다.
- 서버가 공유 스테이지를 허용하기 전에 TTS 및 스티커 출력이 시작되어 미디어나 음악과 겹칠 수 있던 문제를 수정했습니다.
- Relay 포트 변경 후 OBS TTS 및 스티커 소스가 무작정 이동하던 문제를 수정했습니다. 이제 새 서버를 먼저 탐지하며, 시간 제한으로 Browser Source가 멈추지 않습니다.
- 이동 직후 숨기거나 잠그면 위치를 잊던 Windows 미디어 위젯을 수정했습니다.
- 역할 정보가 없어 개인정보 필터 예외를 무시하던 지연 Discord GIF 업데이트를 수정했습니다.
- 주크박스 대기열이 이미 가득 찼을 때 YouTube 트랙 선택이 버려지던 문제를 수정했습니다.
- 시각적 대체 재생이 성공한 뒤에도 Windows TTS 실패를 보고하던 Relay 상태를 수정했습니다.

#### 보안

- YouTube API 키는 로컬 Windows 자격 증명 관리자에 저장되며 저장 후 다시 표시되지 않습니다.
- YouTube 재생 제어는 해당 트랙을 요청한 사용자 또는 Discord 관리자로 제한됩니다.
- 패널과 오버레이 Tauri 창이 서로 다른 권한 집합을 사용합니다. 오버레이 위젯은 더 이상 제어판 명령을 호출할 수 없습니다.
- Discord 봇이 게시하는 오버레이 URL에 더 이상 로컬 Relay 비밀이 포함되지 않습니다. 짧은 페이지는 호스트 전체 쿠키 대신 페이지 전용 비밀을 삽입합니다.
- 더 이상 필터링에 영향을 주지 않던 사용하지 않는 개인정보 임계값 설정을 인터페이스와 구성 스키마에서 제거했습니다.

### 日本語

#### 追加

- 設定可能な Discord チャンネルで YouTube 音楽検索を追加し、1〜5 分の関連結果を最大 15 件表示します。
- 30 秒プレビュー、全曲再生、キュー、OBS と Windows ウィジェット向けの再生中カードを追加しました。
- メディア、ステッカー、TTS 通知、YouTube 再生、Discord オーディオ、TTS 音声向けの統合 OBS ブラウザソース **Relay Visual** と **Relay Audio** を追加しました。移行中は従来のソース URL も利用できます。
- 履歴からのダウンロード、設定可能なグローバルメディアスキップショートカット、ミュージックパネルとドキュメント内の英語 YouTube API セットアップガイドを追加しました。
- インターフェースデザイン **Gridline** と **Lumen**、コンパクト／ダイナミックなサイドバー、折りたたみ可能なデザイン選択を追加しました。
- 検証のたびに既知の重大な問題を確認できるよう、Rust クレートの CI 依存関係監査を追加しました。
- 現在のインターフェース言語で同梱のリリースノートを表示するアプリ内 **Changelog** ページを追加しました。

#### 変更

- メディア、TTS、YouTube 再生が出力スケジュールを共有し、同時再生を避けます。
- YouTube 再生がウィジェットのサウンド設定に従い、必要に応じて Windows オーディオ出力を起こします。
- メディアキャプションはコンパクトなままアクティブなカードに固定され、OBS と Windows ウィジェットの表示を個別に設定できます。
- Windows TTS 通知はより密度の高いトースト（既定 400×104）になり、余白いっぱいに伸びません。生成済みの 980×180 と 480×112 は自動移行し、カスタムサイズは保持されます。
- オーバーレイ移動ラベルとプレビュー文言が、ロシア語、簡体字中国語、韓国語、日本語、インドネシア語を含むすべての Relay インターフェース言語に従います。
- 言語、テーマ、アクセント、フォント倍率は Relay 設定と一緒に保存され、トレイ専用や `--startup` セッションを含む起動時に復元されます。

#### 修正

- 出力ウェイクアップおよび停止／開始の切り替え後の YouTube 再生復元を修正しました。
- 以前のショートカットを捨てずに、スキップショートカット登録の失敗を修正しました。
- キャンセルしたメディアダウンロードが履歴操作を処理中のままにする問題を修正しました。
- GIF 以外の応答を受け入れていた GIF ダウンロードを修正しました。
- 短い WebSocket 切断後に TTS とステッカーのブラウザソースがキューを失う問題を修正しました。
- サーバーが共有ステージを許可する前に TTS とステッカー出力が始まり、メディアや音楽と重なる問題を修正しました。
- Relay のポート変更後に OBS の TTS／ステッカーソースが無確認で遷移する問題を修正しました。新しいサーバーを先に確認し、タイムアウトで Browser Source が停止しないようにしました。
- 移動直後に非表示またはロックすると位置を忘れる Windows メディアウィジェットを修正しました。
- ロール情報がないためにプライバシーフィルターの除外を無視していた遅延 Discord GIF 更新を修正しました。
- ジュークボックスのキューが満杯のときに YouTube 曲の選択が破棄される問題を修正しました。
- 視覚フォールバック成功後も Windows TTS 失敗と報告していた Relay ステータスを修正しました。

#### セキュリティ

- YouTube API キーはローカルの Windows 資格情報マネージャーに保存され、保存後は再表示されません。
- YouTube 再生操作は曲をリクエストしたユーザーまたは Discord 管理者に制限されます。
- パネルとオーバーレイの Tauri ウィンドウは別の権限セットを使います。オーバーレイウィジェットはコントロールパネルコマンドを呼び出せません。
- Discord ボットが投稿するオーバーレイ URL にローカル Relay シークレットは含まれません。短いページはホスト全体の Cookie ではなく、ページ限定のシークレットを注入します。
- フィルタリングに影響しなくなった未使用のプライバシーしきい値設定を、インターフェースと設定スキーマから削除しました。

### Bahasa Indonesia

#### Ditambahkan

- Pencarian musik YouTube di channel Discord yang dapat dikonfigurasi, dengan hingga 15 hasil relevan berdurasi satu hingga lima menit.
- Pratinjau 30 detik, pemutaran penuh, antrean, dan kartu Sedang diputar untuk OBS serta widget Windows.
- Sumber browser OBS terpadu **Relay Visual** dan **Relay Audio** untuk media, stiker, notifikasi TTS, pemutaran YouTube, audio Discord, dan suara TTS. URL sumber lama tetap tersedia selama migrasi.
- Unduhan Riwayat, pintasan loncat media global yang dapat dikonfigurasi, dan panduan YouTube API berbahasa Inggris di panel Musik serta dokumentasi.
- Desain antarmuka **Gridline** dan **Lumen**, tata letak bilah sisi ringkas dan dinamis, serta pemilih desain yang dapat dilipat.
- Audit dependensi Rust di CI agar masalah tingkat tinggi yang diketahui diperiksa pada setiap verifikasi.
- Halaman **Changelog** dalam aplikasi yang menampilkan catatan rilis terbundel dalam bahasa antarmuka saat ini.

#### Diubah

- Pemutaran media, TTS, dan YouTube kini berbagi penjadwalan keluaran agar item tidak saling tumpang tindih.
- Pemutaran YouTube kini mengikuti pengaturan suara widget dan membangunkan keluaran audio Windows jika diperlukan.
- Keterangan media tetap ringkas dan tertambat pada kartu aktif, dengan visibilitas terpisah untuk OBS dan widget Windows.
- Notifikasi TTS Windows kini memakai toast yang lebih padat (default 400×104) dan tidak meregang ke ruang kosong. Ukuran generated 980×180 dan 480×112 dimigrasikan otomatis; ukuran kustom dipertahankan.
- Label geser overlay dan teks pratinjau kini mengikuti semua bahasa antarmuka Relay, termasuk Rusia, Tionghoa Sederhana, Korea, Jepang, dan Indonesia.
- Bahasa, tema, aksen, dan skala font kini disimpan bersama konfigurasi Relay dan dipulihkan saat peluncuran, termasuk sesi baki dan `--startup`.

#### Diperbaiki

- Pemulihan pemutaran YouTube setelah keluaran dibangunkan serta transisi berhenti/mulai.
- Kegagalan pendaftaran pintasan loncat tanpa membuang pintasan sebelumnya.
- Unduhan media yang dibatalkan membuat aksi Riwayat tetap sibuk.
- Unduhan GIF yang menerima respons yang bukan berkas GIF.
- Sumber browser TTS dan stiker yang kehilangan antrean setelah WebSocket terputus sebentar.
- Keluaran TTS dan stiker yang mulai sebelum server memberi izin panggung bersama, sehingga bisa tumpang tindih dengan media atau musik.
- Sumber OBS TTS dan stiker yang pindah port tanpa memeriksa server baru. Sekarang mereka men-probe instance baru dulu, dengan batas waktu, agar Browser Source tidak macet.
- Widget media Windows yang lupa posisi jika disembunyikan atau dikunci tepat setelah dipindah.
- Pembaruan GIF Discord tertunda yang mengabaikan pengecualian filter privasi karena informasi peran hilang.
- Pilihan trek YouTube yang dibuang saat antrean jukebox sudah penuh.
- Status Relay yang melaporkan kegagalan TTS Windows setelah fallback visual berhasil.

#### Keamanan

- Kunci API YouTube disimpan secara lokal di Windows Credential Manager dan tidak ditampilkan lagi setelah disimpan.
- Kontrol pemutaran YouTube dibatasi untuk pengguna yang meminta trek atau administrator Discord.
- Jendela Tauri panel dan overlay kini memakai kumpulan izin terpisah: widget overlay tidak dapat lagi memanggil perintah panel kontrol.
- URL overlay yang diposting bot Discord tidak lagi menyertakan rahasia Relay lokal. Halaman singkat menyuntikkan rahasia khusus halaman, bukan cookie untuk seluruh host.
- Pengaturan ambang privasi yang tidak terpakai dan tidak lagi memengaruhi penyaringan dihapus dari antarmuka dan skema konfigurasi.

## [1.2.7] - 2026-08-14

### English

#### Added

- Added a local settings search bar with `Ctrl+K`, keyboard-accessible results, and page-aware Back and Forward controls.
- Added regional language choices with bundled SVG flags for English (US, UK, and India), French, German, Spanish, and Latin American Spanish while preserving the complete English, French, Spanish, and German dictionaries.
- Added Russian, Simplified Chinese, Korean, Japanese, and Indonesian choices with bundled SVG flags, translated core Relay controls, moderation, privacy protection, custom commands, and the system tray.
- Added eight locally bundled interface fonts: Bricolage Grotesque, DM Sans, Figtree, Inter, JetBrains Mono, Manrope, Poppins, and Space Grotesk. Font files never load from a remote service at runtime.

#### Changed

- Reorganized Moderation into three independent collapsible sections for automatic filtering, manual moderation, and anti-doxxing protection. The existing settings and save behavior are unchanged.
- Completed and corrected the Moderation translations in English, French, Spanish, and German, including protection profiles and input placeholders.

#### Fixed

- HEVC/H.265 Discord videos are now transcoded locally to an H.264-compatible cache when FFmpeg is available, allowing playback in Windows WebView2 widgets. Relay falls back to the original source if conversion cannot complete.

### Français

#### Ajouté

- Ajout d’une barre de recherche locale des réglages avec `Ctrl+K`, de résultats accessibles au clavier et de boutons Retour et Suivant tenant compte de l’historique des pages.
- Ajout de variantes régionales avec des drapeaux SVG embarqués pour l’anglais (États-Unis, Royaume-Uni et Inde), le français, l’allemand, l’espagnol et l’espagnol latino-américain, tout en conservant les dictionnaires complets anglais, français, espagnol et allemand.
- Ajout du russe, du chinois simplifié, du coréen, du japonais et de l’indonésien avec leurs drapeaux SVG embarqués, ainsi que des traductions des contrôles principaux, de la modération, de la protection de la vie privée, des commandes personnalisées et du menu de zone.
- Ajout de huit polices d’interface embarquées localement : Bricolage Grotesque, DM Sans, Figtree, Inter, JetBrains Mono, Manrope, Poppins et Space Grotesk. Aucun fichier de police n’est chargé depuis un service distant à l’exécution.

#### Modifié

- Réorganisation de la page Modération en trois sections repliables indépendantes : filtrage automatique, modération manuelle et protection anti-doxxing. Les réglages existants et leur sauvegarde restent inchangés.
- Traductions de la page Modération complétées et corrigées en anglais, français, espagnol et allemand, notamment pour les profils de protection et les textes indicatifs des champs.

#### Corrigé

- Les vidéos Discord en HEVC/H.265 sont désormais transcodées localement vers un cache compatible H.264 lorsque FFmpeg est disponible, afin de permettre leur lecture dans les widgets Windows WebView2. Relay revient à la source d’origine si la conversion ne peut pas aboutir.

## [1.2.6] - 2026-08-14

### English

#### Fixed

- Discord messages blocked by an automatic filter word are now deleted when **Delete blocked Discord messages** is enabled and Relay has the **Manage Messages** permission, including when the local privacy scan is disabled.

### Français

#### Corrigé

- Les messages Discord bloqués par un mot de filtrage automatique sont désormais supprimés lorsque **Supprimer les messages Discord bloqués** est activé et que Relay possède la permission **Gérer les messages**, même si le scan local de confidentialité est désactivé.

## [1.2.5] - 2026-08-14

### English

#### Added

- Added a fully local anti-doxxing scanner with `SAFE`, `LOW`, `MEDIUM`, `HIGH`, and `CRITICAL` risk levels for Discord text, attachment names, images, and metadata.
- Added configurable Balanced, Strict, and Paranoid protection levels, per-category controls, an automatic block threshold, a local review option, an allowlist, and a private-data protection list.
- Added detection for email addresses, phone numbers, IP addresses, GPS coordinates, postal addresses, IBANs, validated payment cards, license plates, sensitive URLs, administrative-document signals, and user-protected strings.
- Added local Windows OCR and EXIF/GPS inspection for supported images without sending detected content to an external service.
- Added automatic deletion of Discord messages blocked by the selected privacy threshold when the bot has the **Manage Messages** permission.
- Added automatic filter words and phrases with configurable aliases, bounded regular expressions, cautious obfuscation handling, and role-based exemptions.
- Split the Commands page into **Default Commands** and **Custom Commands**, with up to 16 locally configured `/relay <name>` subcommands synchronized for the Relay bot.
- Added predefined Ban, Unban, Kick, Timeout, Remove timeout, Clear messages, Add role, Remove role, and Reply actions with required, optional, or fixed parameters and user, role, channel, and permission restrictions.

#### Security

- Privacy checks now run before sensitive Discord content can enter visible history, WebSocket or OBS output, Windows widgets, media caches, replay, or moderation approval paths.
- Image inspection now enforces trusted Discord CDN hosts, bounded downloads, file-signature checks, size, pixel and frame limits, concurrency limits, and timeouts.
- Privacy logs contain only the risk level, detected category codes, and action; detected addresses, contact details, OCR text, metadata values, and protected strings are not copied into logs.
- Custom moderation actions derive non-disableable Discord permissions, require a one-time 60-second confirmation, recheck authorization and role hierarchy before execution, and suppress mentions in predefined replies.
- Custom-command synchronization validates Discord's candidate schema before local persistence, restores the previous schema if persistence fails, and logs only the command name, action code, and sanitized outcome.

#### Fixed

- Postal addresses are now recognized across punctuation, unusual separators, obfuscated street types, and multiline layouts, including probable addresses without a postcode.
- Intermediate-risk media now uses the existing local moderation queue even when general manual moderation is disabled.
- OCR, malformed metadata, and image-decoder failures no longer interrupt Relay or expose scanned private values in errors.
- Custom Ban and Timeout commands now accept the camelCase action fields sent by the desktop editor while retaining compatibility with previously serialized snake_case fields.
- Custom Ban commands now accept either a current member or a verified Discord user ID that is not yet in the server, allowing a preemptive ban without bypassing hierarchy checks for present members.

### Français

#### Ajouté

- Ajout d'un scanner anti-doxxing entièrement local avec les niveaux de risque `SAFE`, `LOW`, `MEDIUM`, `HIGH` et `CRITICAL` pour le texte Discord, les noms de pièces jointes, les images et leurs métadonnées.
- Ajout des niveaux de protection Balanced, Strict et Paranoid, de catégories configurables, d'un seuil de blocage automatique, d'une option de révision locale, d'une allowlist et d'une liste de données privées à protéger.
- Ajout de la détection des adresses e-mail, numéros de téléphone, adresses IP, coordonnées GPS, adresses postales, IBAN, cartes de paiement validées, plaques d'immatriculation, URL sensibles, indices de documents administratifs et chaînes protégées par l'utilisateur.
- Ajout de l'OCR Windows local et de l'analyse EXIF/GPS pour les images prises en charge, sans transmettre le contenu détecté à un service externe.
- Ajout de la suppression automatique des messages Discord bloqués par le seuil de confidentialité choisi lorsque le bot possède la permission **Gérer les messages**.
- Ajout de mots et expressions filtrés automatiquement avec alias configurables, expressions régulières limitées, gestion prudente de l'obfuscation et exemptions par rôle.
- Séparation de la page Commandes entre **Commandes par défaut** et **Commandes personnalisées**, avec jusqu'à 16 sous-commandes `/relay <nom>` configurées localement et synchronisées pour le bot Relay.
- Ajout des actions prédéfinies Bannir, Débannir, Expulser, Timeout, Retirer le timeout, Effacer des messages, Ajouter un rôle, Retirer un rôle et Réponse, avec paramètres requis, optionnels ou fixes et restrictions par utilisateur, rôle, salon et permission.

#### Sécurité

- Les contrôles de confidentialité s'exécutent désormais avant qu'un contenu Discord sensible puisse atteindre l'historique visible, les sorties WebSocket ou OBS, les widgets Windows, les caches média, le replay ou l'approbation de modération.
- L'analyse des images impose désormais des hôtes CDN Discord approuvés, des téléchargements limités, une vérification de signature, des limites de taille, de pixels et d'images, ainsi que des limites de concurrence et de durée.
- Les journaux de confidentialité contiennent uniquement le niveau de risque, les codes des catégories détectées et l'action ; les adresses, coordonnées, textes OCR, valeurs de métadonnées et chaînes protégées ne sont jamais recopiés.
- Les actions de modération personnalisées imposent les permissions Discord minimales, une confirmation unique de 60 secondes et une nouvelle vérification des autorisations et de la hiérarchie avant exécution ; les mentions sont neutralisées dans les réponses prédéfinies.
- La synchronisation valide le schéma Discord candidat avant la sauvegarde locale, restaure l'ancien schéma si la sauvegarde échoue et ne journalise que le nom de commande, le code d'action et un résultat assaini.

#### Corrigé

- Les adresses postales sont désormais reconnues malgré la ponctuation, les séparateurs inhabituels, les types de voie obfusqués et les présentations multilignes, y compris les adresses probables sans code postal.
- Les médias présentant un risque intermédiaire utilisent désormais la file de modération locale existante même lorsque la modération manuelle générale est désactivée.
- Les échecs OCR, les métadonnées malformées et les erreurs du décodeur d'image n'interrompent plus Relay et n'exposent aucune valeur privée analysée dans les erreurs.
- Les commandes personnalisées Ban et Timeout acceptent désormais les champs camelCase envoyés par l'éditeur tout en restant compatibles avec les anciens champs snake_case sérialisés.
- Les commandes Ban acceptent désormais soit un membre présent, soit l'ID Discord vérifié d'un utilisateur absent du serveur, afin de permettre un bannissement préventif sans contourner la hiérarchie des membres présents.

## [1.2.1] - 2026-07-28

### English

#### Added

- Added `/relay status` to report the live Discord connection, local relay, OBS outputs, queues, and Windows widget state directly in Discord.
- Added `/relay test` for isolated local tests of media, audio, TTS, notifications, and stickers without posting to Discord or adding history entries.
- Added independent options to show up to 180 characters from the Discord media message in OBS and the Windows media widget.
- Added a **Start with Windows** toggle to the system tray; automatic launches now open Relay directly in the tray without showing the control panel.

#### Fixed

- Local media tests and incoming live media now wake the Windows widget before playback so their output is immediately visible.
- The Windows media widget now restores active media after hide/show and avoids WebView2 edge artifacts around videos.
- OBS media and audio outputs no longer overlap during playback.
- Discord invitation links now open reliably in the default web browser instead of File Explorer.

### Français

#### Ajouté

- Ajout de `/relay status` pour afficher directement dans Discord l’état de la connexion, du relais local, des sorties OBS, des files d’attente et des widgets Windows.
- Ajout de `/relay test` pour tester localement les médias, l’audio, le TTS, les notifications et les stickers sans publier dans Discord ni créer d’entrée dans l’historique.
- Ajout d’options indépendantes pour afficher jusqu’à 180 caractères du message Discord associé au média dans OBS et dans le widget média Windows.
- Ajout d’un bouton **Démarrer avec Windows** dans le system tray ; les lancements automatiques ouvrent désormais Relay directement dans le tray sans afficher le panneau de contrôle.

#### Corrigé

- Les tests médias locaux et les nouveaux médias reçus réveillent désormais le widget Windows avant la lecture afin que leur sortie soit immédiatement visible.
- Le widget média Windows restaure le média actif après avoir été masqué puis affiché et évite les artefacts de bordure WebView2 autour des vidéos.
- Les sorties média et audio OBS ne se chevauchent plus pendant la lecture.
- Les liens d’invitation Discord s’ouvrent désormais correctement dans le navigateur web par défaut plutôt que dans l’Explorateur de fichiers.

## [1.2.0] - 2026-07-21

### English

#### Added

- Added three selectable interface styles in Personalization: OpenAI, Anthropic, and Playful Neo-Brutalism, each with light and warm dark variants, keyboard focus, reduced-motion support, and narrow layouts.
- Added Discord guild tags and badges beside author names in TTS notifications when the user enables their primary guild identity.

#### Changed

- The system tray now follows the interface language, theme, and selected design, including translated live status and widget controls.

#### Fixed

- Interface text scaling now applies the selected factor exactly once, preventing oversized or overflowing OpenAI text above 100%.
- Narrow layouts now remain constrained to the viewport when using the Neo-Brutalism design.

### Français

#### Ajouté

- Ajout de trois styles d’interface dans Personnalisation : OpenAI, Anthropic et Playful Neo-Brutalism, avec variantes claire et sombre chaude, focus clavier, réduction des animations et dispositions étroites.
- Ajout des tags et badges de serveur Discord à côté du nom de l’auteur dans les notifications TTS lorsque l’identité du serveur principal est activée par l’utilisateur.

#### Modifié

- Le system tray suit désormais la langue, le thème et le design sélectionné dans l’interface, y compris pour les états en direct et les contrôles des widgets.

#### Corrigé

- La mise à l’échelle du texte applique désormais le facteur sélectionné une seule fois, empêchant les textes OpenAI surdimensionnés ou débordants au-dessus de 100 %.
- Les dispositions étroites restent désormais contenues dans le viewport avec le design Neo-Brutalism.

## [1.1.23] - 2026-07-14

### English

#### Added

- Added an in-app update menu that can check for a new Relay release, then download and install it on confirmation.

#### Fixed

- Audio and video outputs now use an exclusive playback lease so music cannot start while a video is still playing.
- Relay now verifies every downloaded updater installer with an independently stored, pinned signing key before execution.
- Windows notification widgets now grow to the minimum height required by their content scale, preventing the card from disappearing at 135% and above.

### Français

#### Ajouté

- Ajout d’un menu de mise à jour intégré permettant de rechercher une nouvelle version de Relay, puis de la télécharger et de l’installer après confirmation.

#### Corrigé

- Les sorties audio et vidéo utilisent désormais un verrou de lecture exclusif afin que la musique ne démarre pas pendant qu’une vidéo est encore en cours.
- Relay vérifie désormais chaque installateur téléchargé avec une clé de signature épinglée et stockée indépendamment avant toute exécution.
- Les widgets de notifications Windows s’agrandissent désormais jusqu’à la hauteur minimale requise par leur échelle de contenu, ce qui empêche la carte de disparaître à partir de 135 %.

## [1.1.22] - 2026-07-12

### English

#### Added

- Added an Output readiness center showing connection status for visual, audio, TTS, notification, and sticker outputs.
- Added separate OBS, preview, and Windows widget connection tracking for every output.
- Added isolated local output tests that never post to Discord or add entries to Relay history.

#### Changed

- The top-bar OBS source count now excludes internal previews and connection probes.

### Français

#### Ajouté

- Ajout d’un centre d’état des sorties pour les médias visuels, l’audio, le TTS, les notifications et les stickers.
- Ajout d’un suivi distinct des connexions OBS, aperçu et widget Windows pour chaque sortie.
- Ajout de tests locaux isolés par sortie, sans publication Discord ni ajout à l’historique Relay.

#### Modifié

- Le compteur de sources OBS de la barre supérieure exclut désormais les aperçus internes et les sondes de connexion.

## [1.1.21] - 2026-07-12

### English

#### Added

- Added persistent live previews for media and notification output geometry in the Relay panel.
- Added a synchronized top-bar audio player with previous, pause/resume, and skip controls.

#### Fixed

- History now loads a first-frame thumbnail for videos and MP4 GIFs; the Relay logo is used only when loading fails.
- Discord stickers posted in the TTS channel now render in notification cards without speech synthesis.
- Notification content scaling keeps cards inside the viewport.

### Français

#### Ajouté

- Ajout d’aperçus persistants en direct pour régler la géométrie des sorties médias et notifications dans le panneau Relay.
- Ajout d’un mini-lecteur audio synchronisé dans la barre supérieure avec précédent, pause/reprise et suivant.

#### Corrigé

- L’historique charge désormais une miniature de la première image des vidéos et GIF MP4 ; le logo Relay n’est utilisé qu’en cas d’échec.
- Les stickers Discord publiés dans le salon TTS s’affichent désormais dans les cartes de notification sans synthèse vocale.
- L’échelle du contenu des notifications conserve les cartes dans le viewport.

## [1.1.1] - 2026-07-12

### English

#### Added

- Added persistent resizing for the media and notification Windows widgets, with monitor-aware limits and an optional 16:9 ratio for media.
- Added independent crop controls (0–40% per side) and content scaling (50–200%) for media and notifications in OBS and Windows widgets.
- Added configurable Discord bot online status and activity text for custom, playing, listening, watching, and competing activities.
- Added optional media sound in the Windows widget and configurable notification sounds for the widget and OBS.

#### Changed

- Output geometry, crop, scale, and bot presence changes now apply live without reloading overlays or interrupting playback.
- Audio cards, notification cards, and author details now scale cleanly while preserving readable, bounded text.
- Remote artwork downloads now accept only approved HTTPS media hosts and safe redirects.

### Français

#### Ajouté

- Ajout du redimensionnement persistant des widgets Windows médias et notifications, avec des limites adaptées à l’écran et un ratio 16:9 optionnel pour les médias.
- Ajout de contrôles indépendants de rognage (0–40 % par côté) et d’échelle du contenu (50–200 %) pour les médias et notifications dans OBS et les widgets Windows.
- Ajout de la configuration du statut en ligne et de l’activité du bot Discord : personnalisé, joue, écoute, regarde ou participe à une compétition.
- Ajout du son optionnel des médias dans le widget Windows et de sons de notification configurables pour le widget et OBS.

#### Modifié

- Les changements de géométrie, rognage, échelle et présence du bot s’appliquent désormais en direct, sans recharger les overlays ni interrompre la lecture.
- Les cartes audio, cartes de notification et informations d’auteur se redimensionnent proprement avec des textes lisibles et contenus.
- Les téléchargements de pochettes distantes sont désormais limités aux hôtes médias HTTPS approuvés et aux redirections sûres.

## [1.1.0] - 2026-07-12

### English

#### Added

- Added a dedicated Commands page with individual availability switches.
- Added `/relay clear` to delete messages from the configured Discord media and TTS channels without clearing Relay history.
- Added `/relay lock` as a reversible toggle for the configured Discord media channel.
- Added `/relay changelog <channel>` to post the latest release notes, fetched live from GitHub, into a chosen Discord channel.
- Preserved access for Discord administrators and moderation roles while a channel is locked.
- Stored channel permission snapshots locally so unlock restores the previous state.
- Added a dedicated `/stickers` OBS Browser Source with its own 50-item FIFO queue and configurable duration.
- Added Discord PNG, APNG, GIF, and Lottie sticker capture with bounded local caching and a safe visual fallback.
- Added visual rendering for Unicode, static custom, and animated custom emojis in TTS notifications.
- Added a "TTS voice" playback switch; when disabled, TTS messages become silent notifications.
- Added a configurable notification duration (1 to 60 seconds) controlling how long silent TTS notifications stay visible in OBS and the Windows widget.

#### Changed

- Reorganized the playback settings into collapsible categories: display durations, audio and TTS, display.
- Updated the Discord invitation URL with the permissions required for media reading, channel permission overwrites, and message cleanup.
- Documented command permissions in English, French, Spanish, and German.
- Messages containing an emoji now skip speech synthesis while preserving the author and message in the notification output.

#### Fixed

- TTS notifications now appear immediately and stay visible even when audio playback fails in OBS or the widget.
- Fixed visual emoji notifications blocking the following spoken TTS message.
- Added a synthesis timeout so a stalled Windows voice cannot freeze the global TTS queue.
- Added automatic fallback to the default Windows voice and preserved notifications when synthesis fails.
- Fixed `/relay clear` by requiring one explicit Discord channel and a message count from 1 to 1000.
- Fixed delayed Discord GIF embeds that arrived through partial message updates.
- Fixed favorite GIFs represented by Discord as thumbnail-only image embeds.
- Added support for direct thumbnail GIFs without a known GIF provider.

### Français

#### Ajouté

- Ajout d’une page Commandes dédiée avec des interrupteurs de disponibilité individuels.
- Ajout de `/relay clear` pour supprimer des messages des salons Discord médias et TTS configurés sans effacer l’historique Relay.
- Ajout de `/relay lock`, un verrouillage réversible du salon média Discord configuré.
- Ajout de `/relay changelog <channel>` pour publier les dernières notes de version, récupérées en direct depuis GitHub, dans le salon Discord choisi.
- Préservation de l’accès des administrateurs Discord et des rôles de modération pendant le verrouillage d’un salon.
- Sauvegarde locale des instantanés de permissions du salon afin que le déverrouillage restaure l’état précédent.
- Ajout d’une source navigateur OBS `/stickers` dédiée avec sa propre file FIFO de 50 éléments et une durée configurable.
- Ajout de la capture des stickers Discord PNG, APNG, GIF et Lottie avec un cache local borné et un repli visuel sûr.
- Ajout du rendu visuel des emojis Unicode, personnalisés statiques et personnalisés animés dans les notifications TTS.
- Ajout d’un interrupteur « Voix TTS » ; désactivé, les messages TTS deviennent des notifications silencieuses.
- Ajout d’une durée de notification configurable (1 à 60 secondes) contrôlant la visibilité des notifications TTS silencieuses dans OBS et le widget Windows.

#### Modifié

- Réorganisation des réglages de lecture en catégories dépliables : durées d’affichage, audio et TTS, affichage.
- Mise à jour de l’URL d’invitation Discord avec les permissions requises pour la lecture des médias, la modification des permissions de salon et le nettoyage des messages.
- Documentation des permissions des commandes en anglais, français, espagnol et allemand.
- Les messages contenant un emoji sautent désormais la synthèse vocale tout en conservant l’auteur et le message dans la sortie de notification.

#### Corrigé

- Les notifications TTS apparaissent immédiatement et restent visibles même si la lecture audio échoue dans OBS ou le widget.
- Correction des notifications emoji visuelles qui bloquaient le message TTS parlé suivant.
- Ajout d’un délai de synthèse afin qu’une voix Windows bloquée ne puisse plus geler la file TTS globale.
- Ajout d’un repli automatique vers la voix Windows par défaut et préservation des notifications en cas d’échec de synthèse.
- Correction de `/relay clear` en exigeant un salon Discord explicite et un nombre de messages entre 1 et 1000.
- Correction des embeds GIF Discord retardés arrivant via des mises à jour partielles de message.
- Correction des GIF favoris représentés par Discord comme des embeds d’image miniature uniquement.
- Prise en charge des GIF miniatures directs sans fournisseur GIF connu.

## [1.0.0] - 2026-07-12

### English

#### Added

- First public release of Relay for Windows.
- Discord media relay for OBS Browser Sources and Windows widgets.
- Separate media, audio, TTS, and notification outputs.
- Local moderation, playback controls, history, personalization, and multilingual interface.

### Français

#### Ajouté

- Première version publique de Relay pour Windows.
- Relais de médias Discord vers les sources navigateur OBS et les widgets Windows.
- Sorties séparées pour les médias, l’audio, le TTS et les notifications.
- Modération locale, contrôles de lecture, historique, personnalisation et interface multilingue.

[Unreleased]: https://github.com/stealthsrc/relay/compare/v1.3.6...HEAD
[1.3.6]: https://github.com/stealthsrc/relay/compare/v1.3.3...v1.3.6
[1.3.1]: https://github.com/stealthsrc/relay/compare/v1.3.0...v1.3.1
[1.3.0]: https://github.com/stealthsrc/relay/compare/v1.2.7...v1.3.0
[1.2.7]: https://github.com/stealthsrc/relay/compare/v1.2.6...v1.2.7
[1.2.6]: https://github.com/stealthsrc/relay/compare/v1.2.5...v1.2.6
[1.2.5]: https://github.com/stealthsrc/relay/compare/v1.2.1...v1.2.5
[1.2.1]: https://github.com/stealthsrc/relay/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/stealthsrc/relay/compare/v1.1.23...v1.2.0
[1.1.23]: https://github.com/stealthsrc/relay/compare/v1.1.22...v1.1.23
[1.1.22]: https://github.com/stealthsrc/relay/compare/v1.1.21...v1.1.22
[1.1.21]: https://github.com/stealthsrc/relay/releases/tag/v1.1.21
[1.1.1]: https://github.com/stealthsrc/relay/releases/tag/v1.1.1
[1.1.0]: https://github.com/stealthsrc/relay/releases/tag/v1.1.0
[1.0.0]: https://github.com/stealthsrc/relay/releases/tag/v1.0.0

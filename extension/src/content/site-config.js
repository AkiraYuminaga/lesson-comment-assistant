// =============================================================================
// Site integration settings — how the panel reads and fills your school system.
//
// The defaults fit a school system where each student's record opens in a dialog titled
// "<name> Lesson Record", with a "Learning Course" value such as "<course> Lesson 4" and a
// "Completion Status". If your system looks different, adjust the selectors / patterns below
// and reload the extension. Regular expressions are written as strings.
// =============================================================================
globalThis.SCA_SITE = {
  // Panel look
  ui: {
    title: "課堂評語助手",
    subtitle: "填好資料，一鍵生成聯絡簿評語",
    logo: "📝",
    accent: "#f26b1d" // main colour (buttons, highlights)
  },

  // Where the generated comment is pasted. Only a *visible* match is used, so a page that keeps
  // one hidden dialog per student is fine.
  feedbackField: 'textarea#lesson_report_feedback, textarea[name="admission_lesson_report[feedback]"]',

  // The dialog / form around the feedback field; student info is read from inside it.
  dialog: '.modal, [role="dialog"], .modal-dialog',
  dialogTitle: ".modal-title, h1, h2, h3, h4, h5",

  // Student name: group 1 of this pattern, matched against the dialog title.
  studentName: "([^\\s>]+)\\s*Lesson Record",
  // Drop the surname of a 3-character Chinese name (王小明 -> 小明).
  stripSurname: true,

  // Course + lesson: group 1 = course text, group 2 = lesson number, matched against the values
  // and text shown in the dialog. The course text is matched to your courses.json by
  // name / id / aliases, so "Python 進階 Lesson 2" finds the course named or aliased "Python 進階".
  // Course names may contain spaces; the shortest trailing part that matches a course is used.
  lesson: "([^:：|>]+?)\\s*(?:Lesson|L)\\s*(\\d+)",

  // Completion status text in the dialog -> keyword selected automatically (first match wins).
  completion: [
    { match: "\\bnot\\s+completed?\\b|\\bincomplete\\b", tag: "未完成" },
    { match: "Completed in full", tag: "完成" }
  ],

  // Progress buttons in the panel (first one is the default).
  progressOptions: ["進度正常", "進度稍慢", "需要協助"],

  // Keyword library. tone: "warn" highlights the group in amber when selected.
  tagGroups: [
    { title: "學習態度", tags: ["專注", "認真", "仔細", "積極", "有耐心", "主動提問", "樂於分享", "會幫助同學"] },
    { title: "理解與能力", tags: ["理解快", "邏輯清楚", "操作熟練", "會自己除錯", "有創意", "能舉一反三"] },
    { title: "待加強", tags: ["有點分心", "速度較慢", "需要提醒", "需要協助", "比較粗心", "容易放棄"], tone: "warn" },
    { title: "完成狀態", tags: ["完成", "驗收", "未完成", "未驗收"] }
  ],
  // Shown in the collapsed quick row until usage history takes over.
  quickTags: ["專注", "認真", "仔細", "理解快", "主動提問", "完成", "驗收", "未驗收"]
};

// src/utils/textEngine.ts

export interface ResolvedTextLine {
  isRtl: boolean;
  text: string;
  original: string;
}

/**
 * موتور تشخیص هوشمند جهت متن خط‌به‌خط بر پایه کاراکتر، زبان غالب و معکوس‌کننده !
 */
export const resolveLineDirection = (line: string, defaultMajorRtl: boolean): ResolvedTextLine => {
  const trimmed = line.trim();
  const hasInvert = trimmed.startsWith("!");
  const clean = hasInvert ? trimmed.slice(1).trim() : line;

  const rtlRegex = /[\u0600-\u06FF\u0750-\u077F\u08A0-\u08FF\uFB50-\uFDFF\uFE70-\uFEFF]/;
  const hasRtlChar = rtlRegex.test(clean.charAt(0) || "");

  let isRtl = hasRtlChar || defaultMajorRtl;
  if (hasInvert) isRtl = !isRtl;

  return {
    isRtl,
    text: clean,
    original: line,
  };
};

/**
 * کلاس‌های CSS استاندارد برای تایپوگرافی، بدون لگ بودن و اینپوت‌ها
 */
export const commonNodeStyles = {
  container: "group relative rounded-2xl bg-[#1e2025] border transition-colors duration-100 will-change-transform",
  selectedBorder: "border-purple-500 shadow-xl shadow-purple-500/10",
  normalBorder: "border-[#282c36] hover:border-[#383e4d]",
  versionBadge: "nodrag text-center font-mono tracking-tight font-medium text-gray-300 bg-[#16181f] hover:bg-[#222530] focus:bg-[#272b38] focus:text-white focus:outline-none rounded-lg px-2 py-0.5 border border-[#2c3140] transition-colors cursor-text shadow-sm",
  toolbarFloating: "absolute -top-12 left-1/2 -translate-x-1/2 flex items-center gap-1 px-2 py-1.5 rounded-xl bg-[#181a22] border border-[#2b303e] shadow-2xl shadow-black/80 z-50 select-none animate-in fade-in zoom-in-95 duration-100 whitespace-nowrap",
};

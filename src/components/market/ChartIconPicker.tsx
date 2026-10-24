import { useRef, useState } from "react";
import type { DrawingIconCategory } from "@/lib/market/drawingTypes";

export type ChartIconCategory = DrawingIconCategory;
type IconGroup = { label: string; symbols: string[] };

const EMOJI_GROUPS: IconGroup[] = [
  { label: "smiles & people", symbols: "😀 😃 😄 😁 😆 😅 😂 🤣 🥲 ☺️ 😊 😇 🙂 🙃 😉 😌 😍 🥰 😘 😗 😙 😚 😋 😛 😝 😜 🤪 🤨 🧐 🤓 😎 🥸 🤩 🥳 😏 😒 😞 😔 😟 😕 🙁 ☹️ 😣 😖 😫 😩 🥺 😢 😭 😤 😠 😡 🤬 🤯 😳 🥵 🥶 😱 😨 😰 😥 😓 🤗 🤔 🫣 🤭 🫢 🫡 🤫 🫠 🤥 😶 🫥 😐 😑 😬 🙄 😮‍💨 😴 🤤 😪 😵 👋 👍 👎 👏 🙌 🤝 🙏 💪 🫶".split(" ") },
  { label: "animals & nature", symbols: "🐵 🐒 🦍 🦧 🐶 🐕 🦮 🐩 🐺 🦊 🦝 🐱 🐈 🦁 🐯 🐅 🐆 🐴 🫎 🦄 🦓 🦌 🦬 🐮 🐂 🐃 🐄 🐷 🐖 🐗 🐽 🐏 🐑 🐐 🐪 🐫 🦙 🦒 🐘 🦣 🦏 🦛 🐭 🐁 🐀 🐹 🐰 🐇 🐿️ 🦫 🦔 🦇 🐻 🐻‍❄️ 🐨 🐼 🦥 🦦 🦨 🦘 🦡 🐾 🦃 🐔 🐓 🐣 🐤 🐥 🐦 🐧 🕊️ 🦅 🦆 🦉 🦚 🦜 🐸 🐊 🐢 🦎 🐍 🐲 🐉 🦕 🦖 🐳 🐋 🐬 🦭 🐟 🐠 🐡 🦈 🐙 🐚 🪸 🪼 🐌 🦋 🐛 🐜 🐝 🪲 🐞 🦗 🪳 🕷️ 🦂 🦟 🪰 🪱".split(" ") },
  { label: "food & drink", symbols: "🍏 🍎 🍐 🍊 🍋 🍌 🍉 🍇 🍓 🫐 🍈 🍒 🍑 🥭 🍍 🥥 🥝 🍅 🥑 🫒 🥦 🥬 🥒 🌶️ 🫑 🌽 🥕 🫚 🧄 🧅 🥔 🍠 🥐 🥯 🍞 🥖 🥨 🧀 🥚 🍳 🧈 🥞 🧇 🥓 🥩 🍗 🍖 🌭 🍔 🍟 🍕 🫓 🥪 🥙 🧆 🌮 🌯 🫔 🥗 🥘 🫕 🍝 🍜 🍲 🍛 🍣 🍱 🥟 🦪 🍤 🍙 🍚 🍘 🍥 🥠 🥮 🍢 🍡 🍧 🍨 🍦 🥧 🧁 🍰 🎂 🍮 🍭 🍬 🍫 🍿 🍩 🍪 🌰 🥜 🍯 🥛 ☕ 🍵 🧃 🥤 🧋 🍶 🍺 🍻 🥂 🍷 🥃 🍸 🍹 🧉".split(" ") },
  { label: "activities & places", symbols: "⚽ 🏀 🏈 ⚾ 🥎 🎾 🏐 🏉 🎱 🪀 🏓 🏸 🏒 🏑 🥍 🏏 🪃 🥅 ⛳ 🪁 🏹 🎣 🤿 🥊 🥋 🎽 🛹 🛼 🛷 ⛸️ 🥌 🎿 ⛷️ 🏂 🪂 🏋️ 🤼 🤸 ⛹️ 🤺 🤾 🏌️ 🏇 🧘 🏄 🏊 🚴 🚵 🏆 🥇 🥈 🥉 🏅 🎖️ 🏵️ 🎗️ 🎫 🎟️ 🎪 🤹 🎭 🩰 🎨 🎬 🎤 🎧 🎼 🎹 🥁 🎷 🎺 🎸 🪕 🎻 🎲 ♟️ 🎯 🎳 🎮 🎰 🧩 🚗 🚕 🚌 🚎 🏎️ 🚓 🚑 🚒 🚐 🛻 🚚 🚜 🛵 🚲 ✈️ 🚀 🛸 🚁 ⛵ 🚤 🚢 🚉 🚄 🚇 🚲 🏠 🏙️ 🏞️ 🌅 🌄 🌇 🌃 🌌 🌉".split(" ") },
];

const STICKER_GROUPS: IconGroup[] = [
  { label: "trading reactions", symbols: "🚀 📈 📉 💹 💰 💎 🔥 🎯 🧠 👀 ✅ ❌ ⚠️ 🏆 🐂 🐻 🦈 🐋 🦅 🐢 🐇 💪 🙌 👏 🤝 🫡 😎 🤩 🥳 😱 😂 🤔 😴 💡 ⭐ 🌟 ✨ 💥 💫".split(" ") },
  { label: "markets & symbols", symbols: "🟢 🔴 🟡 🔵 🟣 ⚫ ⚪ 🟠 🟤 ⬆️ ⬇️ ↗️ ↘️ ↔️ 🔺 🔻 ◀️ ▶️ ⏫ ⏬ 💲 💱 🪙 🏦 🧾 📊 📉 📈 🗓️ ⏰ ⌛ 🔔 📣 📌 📍 🧭".split(" ") },
];

const ICON_GROUPS: IconGroup[] = [
  { label: "symbols", symbols: "★ ☆ ✦ ✧ ✨ ♡ ♥ ❤ ❣ ❥ ⚑ ⚐ ✓ ✔ ✕ ✖ + − × ÷ = ≠ ≈ ∞ % $ € £ ¥ ₿ ↗ ↘ ↑ ↓ ← → ↔ ◆ ◇ ■ □ ● ○ ◉ ◎ ⌖ ⊕ ⊖ ⊗ ⊙".split(" ") },
  { label: "markers", symbols: "⬆ ⬇ ⬅ ➡ ↖ ↗ ↘ ↙ ⤴ ⤵ ⏫ ⏬ ⏩ ⏪ ⏺ ⏹ ⏸ ▶ ▷ ◀ ◁ ⏭ ⏮ ⚠ ⛔ ⛳ ♻ ☑ ☒ ✚ ✚ ⊙ ⌘ ⌥ ⌫ ⌦ ⌂".split(" ") },
];

const GROUPS: Record<ChartIconCategory, IconGroup[]> = {
  emoji: EMOJI_GROUPS,
  sticker: STICKER_GROUPS,
  icon: ICON_GROUPS,
};

const TABS: { id: ChartIconCategory; label: string }[] = [
  { id: "emoji", label: "Emojis" },
  { id: "sticker", label: "Stickers" },
  { id: "icon", label: "Icons" },
];

interface ChartIconPickerProps {
  onSelect: (category: ChartIconCategory, symbol: string) => void;
}

export function ChartIconPicker({ onSelect }: ChartIconPickerProps): React.ReactElement {
  const [tab, setTab] = useState<ChartIconCategory>("emoji");
  const [activeGroup, setActiveGroup] = useState(EMOJI_GROUPS[0].label);
  const groups = GROUPS[tab];
  const selectedGroup = groups.find((group) => group.label === activeGroup) ?? groups[0];
  const listRef = useRef<HTMLDivElement | null>(null);
  const groupRefs = useRef(new Map<string, HTMLElement>());

  return (
    <div className="chart-icon-picker">
      <div className="chart-icon-picker__categories" role="toolbar" aria-label={`${TABS.find((item) => item.id === tab)?.label} categories`}>
        {groups.map((group) => (
          <button
            key={group.label}
            type="button"
            aria-label={group.label}
            aria-pressed={selectedGroup.label === group.label}
            title={group.label}
            onClick={() => {
              setActiveGroup(group.label);
              const list = listRef.current;
              const section = groupRefs.current.get(`${tab}:${group.label}`);
              if (list && section) list.scrollTo({ top: section.offsetTop - list.offsetTop, behavior: "smooth" });
            }}
            className={`chart-icon-picker__category${selectedGroup.label === group.label ? " is-active" : ""}`}
          >
            {group.symbols[0]}
          </button>
        ))}
      </div>
      <div ref={listRef} className="chart-icon-picker__list" role="listbox" aria-label={TABS.find((item) => item.id === tab)?.label}>
        {groups.map((group) => (
          <section
            key={group.label}
            ref={(element) => {
              const key = `${tab}:${group.label}`;
              if (element) groupRefs.current.set(key, element);
              else groupRefs.current.delete(key);
            }}
            className="chart-icon-picker__group"
            aria-label={group.label}
          >
            <h3 className="chart-icon-picker__heading">{group.label}</h3>
            <div className="chart-icon-picker__grid">
              {group.symbols.map((symbol, index) => (
                <button
                  key={`${group.label}-${symbol}-${index}`}
                  type="button"
                  role="option"
                  aria-label={`${symbol} ${group.label}`}
                  title={symbol}
                  onClick={() => onSelect(tab, symbol)}
                  className="chart-icon-picker__item"
                >
                  {symbol}
                </button>
              ))}
            </div>
          </section>
        ))}
      </div>
      <div className="chart-icon-picker__tabs" role="tablist" aria-label="Icon type">
        {TABS.map((item) => (
          <button
            key={item.id}
            type="button"
            role="tab"
            aria-selected={tab === item.id}
            onClick={() => {
              setTab(item.id);
              setActiveGroup(GROUPS[item.id][0].label);
              listRef.current?.scrollTo({ top: 0 });
            }}
            className={`chart-icon-picker__tab${tab === item.id ? " is-active" : ""}`}
          >
            {item.label}
          </button>
        ))}
      </div>
    </div>
  );
}
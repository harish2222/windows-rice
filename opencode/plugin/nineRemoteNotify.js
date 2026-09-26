// 9Remote OpenCode status plugin (auto-generated)
const base = "http://localhost:2208/api/notify";
const post = (type) => {
  const sid = process.env.NINE_REMOTE_SESSION_ID || "";
  if (!sid) return;
  const url = base + "?type=" + type + "&sessionId=" + encodeURIComponent(sid) + "&tool=opencode";
  try { fetch(url, { signal: AbortSignal.timeout(2000) }).catch(() => {}); } catch {}
};
export const nineRemoteNotify = async () => ({
  "chat.message": async () => post("working"),
  "tool.execute.before": async () => post("working"),
  "tool.execute.after": async () => post("working"),
  event: async ({ event }) => {
    const t = event?.type;
    if (!t) return;
    if (t === "session.idle") return post("done");
    if (t === "permission.asked" || t === "question.asked" || t === "session.error") return post("blocked");
    if (t === "session.compacted" || t === "permission.replied" || t === "question.replied") return post("working");
  },
});

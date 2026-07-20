// Auto-update qua tauri-plugin-updater: check GitHub Releases latest.json,
// tải + cài (artifact ký minisign, verify bằng pubkey trong tauri.conf.json),
// rồi relaunch. Object `Update` phải giữ lại giữa check → install nên cache module-level.
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateInfo = { version: string; notes: string | null };

let pending: Update | null = null;

/** Hỏi server có bản mới không. Trả null nếu đang là bản mới nhất. */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  const up = await check();
  if (up) {
    pending = up;
    return { version: up.version, notes: up.body ?? null };
  }
  pending = null;
  return null;
}

/** Tải + cài bản đã tìm thấy ở `checkForUpdate`, báo tiến độ %, xong thì relaunch. */
export async function downloadAndInstall(onProgress: (pct: number | null) => void): Promise<void> {
  if (!pending) throw new Error("Chưa tìm thấy bản cập nhật — hãy kiểm tra lại");
  let total = 0;
  let got = 0;
  await pending.downloadAndInstall((e) => {
    if (e.event === "Started") {
      total = e.data.contentLength ?? 0;
      onProgress(total ? 0 : null);
    } else if (e.event === "Progress") {
      got += e.data.chunkLength;
      onProgress(total ? Math.min(99, Math.round((got * 100) / total)) : null);
    } else if (e.event === "Finished") {
      onProgress(100);
    }
  });
  await relaunch();
}

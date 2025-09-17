/* global guess */

if (typeof importScripts !== "undefined") {
  self.importScripts("guess.js");
}

// 配置对象
let config = {
  regexPattern: [],
  outputPath: null,
  serverPort: 8080,
  waitTime: 1000,
  sendToRust: false,
  openDownload: true, // 新增：默认开启
};

const badge = (tabId, title, text = "E") => {
  chrome.action.setBadgeText({
    tabId,
    text,
  });
  chrome.action.setTitle({
    tabId,
    title,
  });
  chrome.action.setBadgeBackgroundColor({
    tabId,
    color: "red",
  });
};

const actives = new Map();

// 发送事件到内容脚本
function sendEventToContentScript(tabId, event, data) {
  chrome.tabs
    .sendMessage(tabId, {
      type: "LITE_IMAGE_DOWNLOADER_EVENT",
      event: event,
      data: data,
    })
    .catch((e) => {
      // 忽略错误（可能是标签页没有内容脚本）
    });
}

// 与Rust服务器通信的函数
async function communicateWithRust() {
  try {
    console.log("Attempting to connect to Rust server...");
    const response = await fetch(
      `http://localhost:${config.serverPort}/config`
    );
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const rustConfig = await response.json();

    // 更新配置
    if (rustConfig.regexPattern) config.regexPattern = rustConfig.regexPattern;
    if (rustConfig.outputPath) config.outputPath = rustConfig.outputPath;
    if (rustConfig.serverPort) config.serverPort = rustConfig.serverPort;
    if (rustConfig.waitTime) config.waitTime = rustConfig.waitTime;
    if (rustConfig.sendToRust !== undefined)
      config.sendToRust = rustConfig.sendToRust;
    if (rustConfig.openDownload !== undefined)
      // 新增
      config.openDownload = rustConfig.openDownload;

    console.log("Updated config from Rust:", config);
    return true;
  } catch (e) {
    console.log("Could not connect to Rust server, using default config", e);
    return false;
  }
}

// 发送数据到Rust服务器的函数
async function sendToRustServer(filename, data, mimeType, url, tabId) {
  try {
    const formData = new FormData();
    const blob = new Blob([data], { type: mimeType });
    formData.append("file", blob, filename);
    formData.append("filename", filename);
    formData.append("url", url);
    formData.append("mimeType", mimeType);

    const response = await fetch(
      `http://localhost:${config.serverPort}/upload`,
      {
        method: "POST",
        body: formData,
      }
    );

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    console.log("Data sent to Rust server successfully:", filename);
    sendEventToContentScript(tabId, "fileUploaded", { filename, url });
    return true;
  } catch (e) {
    console.error("Error sending data to Rust server:", e);
    sendEventToContentScript(tabId, "uploadError", {
      filename,
      url,
      error: e.message,
    });
    return false;
  }
}

// 清理文件名中的非法字符
function sanitizeFilename(filename) {
  // 移除路径分隔符和非法字符
  return filename.replace(/[\\/:*?"<>|]/g, "_");
}

// 检查URL是否匹配任一正则表达式模式
function matchesAnyPattern(url, patterns) {
  // 严格校验patterns是否为数组，非数组则视为无匹配规则（允许所有URL）
  if (!Array.isArray(patterns)) {
    console.warn("Patterns must be an array, allowing all URLs");
    return true;
  }

  // 扁平化数组（处理嵌套数组的情况）
  const flatPatterns = patterns.flat(Infinity);

  // 空数组直接允许所有URL
  if (flatPatterns.length === 0) {
    console.log("No patterns provided, allowing all URLs");
    return true;
  }

  console.log("Checking URL against patterns:", url, flatPatterns);

  for (const pattern of flatPatterns) {
    try {
      // 确保模式是字符串，非字符串类型直接跳过
      if (typeof pattern !== "string") {
        console.warn("Skipping non-string pattern:", pattern);
        continue;
      }

      const patternStr = pattern.trim();
      if (!patternStr) {
        console.log("Skipping empty pattern");
        continue; // 跳过空模式
      }

      // 直接使用字符串模式创建正则表达式
      // 注意：这里不需要转义，因为模式已经是正确的正则表达式字符串
      const regex = new RegExp(patternStr);
      
      const isMatch = regex.test(url);
      console.log(`Pattern test: ${patternStr} → ${isMatch}`);

      if (isMatch) {
        console.log("URL matches pattern:", patternStr);
        return true;
      }
    } catch (e) {
      console.error(`Invalid regex pattern "${pattern}":`, e.message);
    }
  }

  console.log("URL does not match any pattern");
  return false;
}

// 修改后的 download 函数
const download = async (tabId) => {
  console.log("Starting download process for tab:", tabId);

  // 检查是否还有活动的下载
  if (!actives.has(tabId)) {
    return;
  }

  const { controller, targets } = actives.get(tabId);

  // 如果已中止，则返回
  if (controller.signal.aborted) {
    return detach(tabId);
  }

  // 检查是否正在下载，避免重复执行
  if (actives.get(tabId).downloading) {
    console.log("Download already in progress for tab:", tabId);
    return;
  }

  // 设置下载状态
  actives.get(tabId).downloading = true;

  console.log("Number of targets:", targets.size);

  // 创建一个数组来存储所有需要处理的资源
  let allResources = [];
  for (const [, entries] of targets) {
    // 检查 entries 是否存在且是 Set 类型
    if (entries && typeof entries === "object" && entries instanceof Set) {
      allResources = allResources.concat(Array.from(entries));
    }
  }

  console.log(`Total resources to process: ${allResources.length}`);

  // 如果没有资源需要处理，则完成下载
  if (allResources.length === 0) {
    actives.get(tabId).downloading = false;

    // 检查是否所有框架都已处理
    if (checkAllFramesProcessed(tabId)) {
      actives.get(tabId).finished = setTimeout(() => {
        if (actives.has(tabId)) {
          console.info("[finished]", tabId, actives.get(tabId));
          sendEventToContentScript(tabId, "downloadCompleted", {
            total: actives.get(tabId).total,
            downloaded: actives.get(tabId).counter,
          });
          detach(tabId);
        }
      }, 1000);
    }
    return;
  }

  // 使用for循环迭代处理所有资源
  for (const entry of allResources) {
    console.log("Processing resource:", entry.resource.url);

    if (controller.signal.aborted || !actives.has(tabId)) {
      return detach(tabId);
    }

    // 从对应的entries中删除这个资源
    if (entry.entries && entry.entries.delete) {
      entry.entries.delete(entry);
    }

    actives.get(tabId).counter += 1;
    const currentCounter = actives.get(tabId).counter;
    const currentTotal = actives.get(tabId).total;
    const progress = Math.round((currentCounter / currentTotal) * 100);

    // 更新进度显示
    chrome.action.setBadgeText({
      tabId: tabId,
      text: progress + "%",
    });

    // 发送进度事件
    sendEventToContentScript(tabId, "downloadProgress", {
      current: currentCounter,
      total: currentTotal,
      progress: progress,
      url: entry.resource.url,
    });

    try {
      // 正则表达式过滤 - 使用新的多正则匹配函数
      console.log(
        "Checking URL against patterns:",
        entry.resource.url,
        "Patterns:",
        config.regexPattern
      );

      const matches = matchesAnyPattern(
        entry.resource.url,
        config.regexPattern
      );
      console.log(
        "Pattern test result:",
        matches,
        "for URL:",
        entry.resource.url
      );

      if (!matches) {
        console.log(
          "Skipping resource due to pattern filter:",
          entry.resource.url
        );
        continue; // 跳过此资源，继续处理下一个
      }

      // 获取资源内容
      const content = await chrome.debugger.sendCommand(
        entry.target,
        "Page.getResourceContent",
        {
          frameId: entry.frameId,
          url: entry.resource.url,
        }
      );

      if (controller.signal.aborted || !actives.has(tabId)) {
        return detach(tabId);
      }

      const url = `data:${entry.resource.mimeType}${
        content.base64Encoded ? ";base64" : ""
      },${content.content}`;
      const meta = {};
      guess(
        {
          url: entry.resource.url,
          headers: new Map([
            ["Content-Size", entry.resource.contentSize],
            ["Content-Type", entry.resource.mimeType],
          ]),
        },
        meta
      );

      // 清理文件名
      meta.name = sanitizeFilename(meta.name);

      // 如果配置了发送到Rust服务器
      if (config.sendToRust) {
        // 解码内容
        const fileData = content.base64Encoded
          ? Uint8Array.from(atob(content.content), (c) => c.charCodeAt(0))
          : new TextEncoder().encode(content.content);

        // 发送到Rust服务器
        const success = await sendToRustServer(
          meta.name + "." + meta.ext,
          fileData,
          entry.resource.mimeType,
          entry.resource.url,
          tabId
        );

        if (!success) {
          console.warn(
            "Failed to send to Rust, falling back to local download"
          );
          // 如果发送失败，回退到本地下载
          await chrome.downloads.download({
            url,
            conflictAction: "uniquify",
          });
        }
      } else {
        // 本地下载
        try {
          // 使用配置的输出路径
          let filename = meta.name + "." + meta.ext;
          if (config.outputPath) {
            // 注意：Chrome扩展不能直接指定绝对路径，只能指定相对于下载目录的路径
            filename = config.outputPath + "/" + filename;
          }
          await chrome.downloads.download({
            filename,
            url,
            conflictAction: "uniquify",
          });
        } catch (e) {
          console.error("Download failed, trying without custom path:", e);
          await chrome.downloads.download({
            url,
            conflictAction: "uniquify",
          });
        }
      }

      // 使用配置的等待时间
      await new Promise((resolve) => setTimeout(resolve, config.waitTime));
    } catch (e) {
      console.error("Error processing resource:", e, entry.resource.url);
      sendEventToContentScript(tabId, "downloadError", {
        url: entry.resource.url,
        error: e.message,
      });
    }
  }

  // 所有资源处理完成
  actives.get(tabId).downloading = false;

  // 检查是否所有框架都已处理
  if (checkAllFramesProcessed(tabId)) {
    actives.get(tabId).finished = setTimeout(() => {
      if (actives.has(tabId)) {
        console.info("[finished]", tabId, actives.get(tabId));
        sendEventToContentScript(tabId, "downloadCompleted", {
          total: actives.get(tabId).total,
          downloaded: actives.get(tabId).counter,
        });
        detach(tabId);
      }
    }, 1000);
  }
};

// 修改 collect 函数，确保资源被正确添加
const collect = (tabId, target, frameTree) => {
  console.info("[collecting]", tabId, frameTree.frame.url);

  const { targets, frames, processedUrls } = actives.get(tabId);
  let entries = targets.get(target);

  // 如果这个目标还没有entries，创建一个新的Set
  if (!entries) {
    entries = new Set();
    targets.set(target, entries);
  }

  const getAllResources = (frameTree) => {
    actives.get(tabId).frames = frames + 1;

    for (const resource of frameTree.resources) {
      // 扩展图像类型检测
      const imageExtensions = /\.(png|jpg|jpeg|gif|webp|bmp|svg|ico)$/i;
      const isImage =
        resource.type === "Image" || imageExtensions.test(resource.url);

      if (isImage) {
        // 使用URL和内容大小作为唯一标识
        const resourceKey = `${resource.url}_${
          resource.contentSize || "unknown"
        }`;

        // 检查是否已经处理过这个资源
        if (processedUrls.has(resourceKey)) {
          console.log("Skipping duplicate resource:", resourceKey);
          continue;
        }

        processedUrls.add(resourceKey);

        // 创建entry对象，包含target引用
        const entry = {
          target: target,
          frameId: frameTree.frame.id,
          resource: resource,
          entries: entries,
        };

        entries.add(entry);
        actives.get(tabId).total = actives.get(tabId).total + 1;
        console.log("Added resource:", resource.url);
      }
    }
    for (const child of frameTree.childFrames || []) {
      getAllResources(child);
    }
  };
  getAllResources(frameTree);

  sendEventToContentScript(tabId, "downloadStarted", {
    total: actives.get(tabId).total,
  });

  // 使用setTimeout确保资源添加完成后再开始下载
  setTimeout(() => download(tabId), 100);
};

// 添加辅助函数检查所有框架是否已处理
function checkAllFramesProcessed(tabId) {
  if (!actives.has(tabId)) return false;

  const { targets, frames } = actives.get(tabId);
  let processedFrames = 0;

  for (const [, entries] of targets) {
    processedFrames++;
  }

  return processedFrames >= frames;
}

chrome.debugger.onEvent.addListener(async (source, method, params) => {
  if (method === "Target.attachedToTarget") {
    if (actives.has(source.tabId)) {
      const { targets } = actives.get(source.tabId);

      const target = {
        tabId: source.tabId,
        sessionId: params.sessionId,
      };
      const entries = new Set();
      targets.set(target, entries);
      await chrome.debugger.sendCommand(target, "Page.enable");
      const result = await chrome.debugger.sendCommand(
        target,
        "Page.getResourceTree"
      );
      collect(source.tabId, target, result.frameTree);
    }
  }
});

const detach = (tabId) => {
  if (actives.has(tabId)) {
    const { controller, finished } = actives.get(tabId);
    controller.abort();
    clearTimeout(finished);

    actives.delete(tabId);
    chrome.debugger.detach({ tabId }).catch(() => {});
    chrome.action.setBadgeText({
      tabId: tabId,
      text: "",
    });

    sendEventToContentScript(tabId, "downloadStopped", {});
  }
};

const attach = async (tab) => {
  // 先尝试从Rust获取配置
  await communicateWithRust();

  // 新增：检查是否允许开启下载功能
  if (!config.openDownload) {
    console.log("Download is disabled by Rust config");
    badge(tab.id, "Download disabled", "X");
    sendEventToContentScript(tab.id, "downloadError", {
      error: "Download function is disabled by configuration",
    });
    return;
  }

  const controller = new AbortController();
  const entries = new Set();
  const targets = new Map();
  const processedUrls = new Set(); // 新增：用于跟踪已处理的URL

  actives.set(tab.id, {
    controller,
    targets,
    frames: 0,
    counter: 0,
    total: 0,
    downloading: false,
    finished: -1,
    processedUrls, // 新增
  });

  const target = { tabId: tab.id };
  targets.set(target, entries);

  try {
    await chrome.debugger.attach(target, "1.3");
    await chrome.debugger.sendCommand(target, "Target.setAutoAttach", {
      autoAttach: true,
      waitForDebuggerOnStart: false,
      flatten: true,
    });
    await chrome.debugger.sendCommand(target, "Page.enable");
    const result = await chrome.debugger.sendCommand(
      target,
      "Page.getResourceTree"
    );
    collect(tab.id, target, result.frameTree);
  } catch (e) {
    console.error(e);
    badge(tab.id, "Error: " + e.message, "E");
    sendEventToContentScript(tab.id, "downloadError", {
      error: e.message,
    });
  }
};

// 处理来自内容脚本的消息
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.type === "LITE_IMAGE_DOWNLOADER") {
    const { action, data } = request;

    switch (action) {
      case "startDownload":
        // 新增：检查是否允许开启下载
        if (!config.openDownload) {
          sendResponse({
            success: false,
            error: "Download function is disabled by configuration",
          });
          return true;
        }

        if (sender.tab) {
          // 更新配置
          if (data) {
            if (data.regexPattern) config.regexPattern = data.regexPattern;
            if (data.outputPath) config.outputPath = data.outputPath;
            if (data.sendToRust !== undefined)
              config.sendToRust = data.sendToRust;
            if (data.waitTime) config.waitTime = data.waitTime;

            sendEventToContentScript(sender.tab.id, "configUpdated", config);
          }

          // 开始下载
          if (actives.has(sender.tab.id)) {
            detach(sender.tab.id);
          }
          attach(sender.tab);

          sendResponse({ success: true, data: "Download started" });
        } else {
          sendResponse({ success: false, error: "No tab information" });
        }
        break;

      case "stopDownload":
        if (sender.tab && actives.has(sender.tab.id)) {
          detach(sender.tab.id);
          sendResponse({ success: true, data: "Download stopped" });
        } else {
          sendResponse({ success: false, error: "No active download" });
        }
        break;

      case "getStatus":
        if (sender.tab) {
          const status = actives.has(sender.tab.id) ? "active" : "inactive";
          const activeInfo = actives.get(sender.tab.id);
          sendResponse({
            success: true,
            data: {
              status,
              activeInfo: activeInfo
                ? {
                    counter: activeInfo.counter,
                    total: activeInfo.total,
                    downloading: activeInfo.downloading,
                  }
                : null,
              // 新增：返回当前 openDownload 状态
              isDownloadEnabled: config.openDownload,
            },
          });
        } else {
          sendResponse({ success: false, error: "No tab information" });
        }
        break;

      case "fetchLatestConfig":
        if (sender.tab) {
          // 调用现有方法从服务器获取配置
          communicateWithRust().then((success) => {
            if (success) {
              // 配置更新后，通知当前标签页的内容脚本
              sendEventToContentScript(sender.tab.id, "configUpdated", config);
              sendResponse({
                success: true,
                data: {
                  message: "Config updated successfully",
                  config: config, // 返回更新后的配置
                },
              });
            } else {
              sendResponse({
                success: false,
                error: "Failed to fetch latest config from server",
              });
            }
          });
        } else {
          sendResponse({ success: false, error: "No tab information" });
        }
        break;

      default:
        sendResponse({ success: false, error: "Unknown action" });
    }

    return true; // 保持消息通道开放以支持异步响应
  }
});

chrome.action.onClicked.addListener((tab) => {
  // 新增：检查是否允许开启下载
  if (!config.openDownload) {
    badge(tab.id, "Download disabled", "X");
    return;
  }

  if (actives.has(tab.id)) {
    detach(tab.id);
  } else {
    attach(tab);
  }
});

// 添加一个函数来定期更新配置（可选）
setInterval(async () => {
  // 如果有活动的下载，定期更新配置
  if (actives.size > 0) {
    await communicateWithRust();
  }
}, 30000); // 每30秒更新一次配置

/* FAQs & Feedback */
{
  const {
    management,
    runtime: { onInstalled, setUninstallURL, getManifest },
    storage,
    tabs,
  } = chrome;
  if (navigator.webdriver !== true) {
    const { homepage_url: page, name, version } = getManifest();
    onInstalled.addListener(({ reason, previousVersion }) => {
      management.getSelf(
        ({ installType }) =>
          installType === "normal" &&
          storage.local.get(
            {
              faqs: true,
              "last-update": 0,
            },
            (prefs) => {
              if (reason === "install" || (prefs.faqs && reason === "update")) {
                const doUpdate =
                  (Date.now() - prefs["last-update"]) / 1000 / 60 / 60 / 24 >
                  45;
                if (doUpdate && previousVersion !== version) {
                  tabs.query({ active: true, lastFocusedWindow: true }, (tbs) =>
                    tabs.create({
                      url:
                        page +
                        "?version=" +
                        version +
                        (previousVersion ? "&p=" + previousVersion : "") +
                        "&type=" +
                        reason,
                      active: reason === "install",
                      ...(tbs && tbs.length && { index: tbs[0].index + 1 }),
                    })
                  );
                  storage.local.set({ "last-update": Date.now() });
                }
              }
            }
          )
      );
    });
    setUninstallURL(
      page +
        "?rd=feedback&name=" +
        encodeURIComponent(name) +
        "&version=" +
        version
    );
  }
}

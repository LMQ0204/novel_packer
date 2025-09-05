// ==UserScript==
// @name         显示隐藏内容脚本（SingleFile修复版）
// @namespace    https://example.com
// @version      3.1
// @description  全消息通信实现：显示隐藏内容+移除广告+等待下载完成（适配下载状态控制）
// @author       您的名称
// @match        *://*/*
// @grant        none
// ==/UserScript==

(function () {
  if (window.self !== window.top) return;

  // 初始化SingleFile
  dispatchEvent(new CustomEvent("single-file-user-script-init"));

  // 配置参数（基于扩展消息协议）
  const CONFIG = {
    WAIT_AFTER_EXECUTION: 1,
    SCROLL_DELAY: 1000,
    SCROLL_BEHAVIOR: "smooth",
    SCROLL_SPEED: 2500, // 默认1000像素/秒
    MAX_WAIT_TIME: 120000, // 全局超时（120秒）
    DOWNLOAD_TIMEOUT: 30000, // 下载超时（30秒）
    API_CHECK_TIMEOUT: 2000, // API检测超时（2秒）
    AD_SELECTORS: [
      "ins.adsbygoogle",
      'div[class*="ad"]',
      'div[id*="ad"]',
      'iframe[src*="ad"]',
    ],
    HIDDEN_SELECTORS: [
      "#mlfy_main_text #TextContent",
      '#mlfy_main_text [style*="display: none"]',
      '#mlfy_main_text [style*="display:none"]',
      ".hidden-content",
      "#hidden-images",
      '[class*="hidden"]',
      '[style*="display: none"]',
    ],
    // 扩展消息协议（匹配inject.js定义）
    MSG_TYPES: {
      REQUEST: "LITE_IMAGE_DOWNLOADER",
      RESPONSE: "LITE_IMAGE_DOWNLOADER_RESPONSE",
      EVENT: "LITE_IMAGE_DOWNLOADER_EVENT",
    },
    // 扩展事件类型（补充CONFIG_UPDATED事件）
    EXT_EVENTS: {
      DOWNLOAD_STARTED: "downloadStarted",
      DOWNLOAD_PROGRESS: "downloadProgress",
      DOWNLOAD_COMPLETED: "downloadCompleted",
      DOWNLOAD_ERROR: "downloadError",
      CONFIG_UPDATED: "configUpdated", // 新增：匹配inject.js的配置更新事件
    },
  };

  // 生成唯一ID（用于消息跟踪）
  function generateId() {
    return `usr_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
  }

  // 【核心】发送消息给扩展（通用函数）
  function sendExtensionMessage(action, data = {}) {
    return new Promise((resolve, reject) => {
      const msgId = generateId();
      const timeout = setTimeout(() => {
        reject(new Error(`消息超时（action: ${action}）`));
      }, CONFIG.API_CHECK_TIMEOUT);

      // 监听扩展响应
      const handleResponse = (event) => {
        if (
          event.source !== window ||
          event.data.type !== CONFIG.MSG_TYPES.RESPONSE ||
          event.data.id !== msgId
        ) {
          return;
        }

        clearTimeout(timeout);
        window.removeEventListener("message", handleResponse);

        if (event.data.success) {
          resolve(event.data.data);
        } else {
          reject(new Error(`扩展响应失败: ${event.data.error || "未知错误"}`));
        }
      };

      window.addEventListener("message", handleResponse);

      // 发送消息（匹配inject.js的消息格式）
      window.postMessage(
        {
          type: CONFIG.MSG_TYPES.REQUEST,
          id: msgId,
          action: action,
          data: data,
        },
        "*"
      );
    });
  }

  // 【核心】监听扩展事件（如下载完成/错误）
  function listenExtensionEvent(eventName) {
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        window.removeEventListener("message", handleEvent);
        reject(new Error(`事件监听超时（event: ${eventName}）`));
      }, CONFIG.DOWNLOAD_TIMEOUT);

      const handleEvent = (event) => {
        if (
          event.source !== window ||
          event.data.type !== CONFIG.MSG_TYPES.EVENT ||
          event.data.event !== eventName
        ) {
          return;
        }

        clearTimeout(timeout);
        window.removeEventListener("message", handleEvent);
        resolve(event.data.data);
      };

      window.addEventListener("message", handleEvent);
    });
  }

  // 检测扩展是否可用并获取当前下载状态
  async function checkExtensionAndStatus() {
    try {
      // 1. 先强制刷新最新配置（从扩展/Rust服务器获取）
      await sendExtensionMessage("fetchLatestConfig");
      // 2. 刷新状态（确保获取到最新的下载启用状态）
      await sendExtensionMessage("getStatus");
      // 3. 再次获取状态，确保是最新值
      const status = await sendExtensionMessage("getStatus");
      console.log(
        "扩展API可用，当前下载状态:",
        status.isDownloadEnabled ? "启用" : "禁用"
      );
      return {
        available: true,
        isDownloadEnabled: status.isDownloadEnabled ?? true,
      };
    } catch (error) {
      console.log("扩展API不可用:", error.message);
      return { available: false, isDownloadEnabled: false };
    }
  }

  // 检测滚动是否完成
  function isScrolling() {
    return new Promise((resolve) => {
      let lastScrollTop = window.pageYOffset;
      let scrollTimeout;

      const checkScroll = () => {
        const scrollTop = window.pageYOffset;

        if (scrollTop !== lastScrollTop) {
          lastScrollTop = scrollTop;
          clearTimeout(scrollTimeout);
          scrollTimeout = setTimeout(checkScroll, 100);
        } else {
          resolve();
        }
      };

      scrollTimeout = setTimeout(checkScroll, 100);
    });
  }

  // 自定义滚动函数
  function scrollToPosition(position) {
    return new Promise((resolve) => {
      const start = window.pageYOffset;
      const distance = position - start;
      const duration = (Math.abs(distance) / CONFIG.SCROLL_SPEED) * 1000; // 计算持续时间（毫秒）
      let startTime = null;

      function animation(currentTime) {
        if (startTime === null) startTime = currentTime;
        const timeElapsed = currentTime - startTime;
        const scrollY = easeInOutQuad(timeElapsed, start, distance, duration);
        window.scrollTo(0, scrollY);

        if (timeElapsed < duration) {
          requestAnimationFrame(animation);
        } else {
          // 等待滚动完全停止
          setTimeout(() => {
            isScrolling().then(resolve);
          }, 100);
        }
      }

      // 缓动函数
      function easeInOutQuad(t, b, c, d) {
        t /= d / 2;
        if (t < 1) return (c / 2) * t * t + b;
        t--;
        return (-c / 2) * (t * (t - 2) - 1) + b;
      }

      requestAnimationFrame(animation);
    });
  }

  // 滚动工具函数 - 保持与您的设计一致
  function scrollToBottom() {
    // addLog("开始滚动到底部");
    if (CONFIG.SCROLL_BEHAVIOR === "auto") {
      window.scrollTo({
        top: document.body.scrollHeight,
        behavior: "auto",
      });
      return Promise.resolve();
    } else {
      return scrollToPosition(document.body.scrollHeight);
    }
  }

  function scrollToTop() {
    // addLog("开始滚动到顶部");
    if (CONFIG.SCROLL_BEHAVIOR === "auto") {
      window.scrollTo({
        top: 0,
        behavior: "auto",
      });
      return Promise.resolve();
    } else {
      return scrollToPosition(0);
    }
  }

  // // 滚动工具函数
  // function scrollToBottom() {
  //   window.scrollTo({
  //     top: document.body.scrollHeight,
  //     behavior: CONFIG.SCROLL_BEHAVIOR,
  //   });
  // }

  // function scrollToTop() {
  //   window.scrollTo({
  //     top: 0,
  //     behavior: CONFIG.SCROLL_BEHAVIOR,
  //   });
  // }

  // 显示隐藏内容 + 移除广告（返回Promise）
  function showHiddenElements() {
    return new Promise((resolve) => {
      console.log("开始显示隐藏内容并移除广告...");

      // 使用MutationObserver等待DOM操作完成
      const observer = new MutationObserver(() => {
        observer.disconnect();
        resolve();
      });

      observer.observe(document.body, {
        childList: true,
        subtree: true,
        attributes: true,
      });

      // 移除广告
      CONFIG.AD_SELECTORS.forEach((selector) => {
        document.querySelectorAll(selector).forEach((el) => {
          el.style.display = "none";
          console.log(`已隐藏广告元素: ${selector}`);
        });
      });

      // 显示隐藏内容
      CONFIG.HIDDEN_SELECTORS.forEach((selector) => {
        document.querySelectorAll(selector).forEach((el) => {
          el.style.display = "block";
          console.log(`已显示隐藏元素: ${selector}`);
        });
      });

      // 如果没有找到任何元素，立即resolve
      setTimeout(resolve, 0);
    });
  }

  // 封装为返回 Promise 的函数
  function addOriginalSrcToImagesAsync() {
    return new Promise((resolve) => {
      document.querySelectorAll("img").forEach((img) => {
        if (img.src) {
          img.setAttribute("data-original-src", img.src);
          console.log(`已为图片添加属性: ${img.src}`);
        }
      });
      resolve(); // 同步操作完成后立即 resolve
    });
  }

  // 启动下载并等待完成（增加下载状态检查）
  async function startDownloadAndWait() {
    try {
      // 1. 再次确认下载状态（防止配置在检查后发生变更）
      const status = await sendExtensionMessage("getStatus");
      if (!status.isDownloadEnabled) {
        throw new Error("下载功能已被禁用，请检查扩展配置");
      }

      // 2. 启动下载
      const startRes = await sendExtensionMessage("startDownload", {
        regexPattern: null,
        outputPath: null,
        sendToRust: false,
        waitTime: 1000,
      });
      console.log("扩展已启动下载:", startRes);

      // 3. 监听下载完成事件
      const completeData = await listenExtensionEvent(
        CONFIG.EXT_EVENTS.DOWNLOAD_COMPLETED
      );
      console.log("下载完成，数据:", completeData);
      return completeData;
    } catch (error) {
      // 专门处理下载功能禁用的错误
      if (
        error.message.includes("Download function is disabled") ||
        error.message.includes("下载功能已被禁用")
      ) {
        console.error("下载失败：功能已禁用");
        throw new Error("下载功能已禁用，无法执行下载操作");
      }
      // 处理超时
      if (error.message.includes("事件监听超时")) {
        await sendExtensionMessage("stopDownload").catch(() => {});
        throw new Error(`下载超时（超过${CONFIG.DOWNLOAD_TIMEOUT}ms）`);
      }
      throw error;
    }
  }

  // 替代图片处理方案（扩展不可用或下载禁用时）
  function useAlternativeImageProcessing() {
    console.log("使用替代图片处理方案...");

    // 修复相对路径图片
    document.querySelectorAll("img").forEach((img) => {
      if (img.src && !img.src.startsWith("http")) {
        try {
          img.src = new URL(img.src, window.location.href).href;
          console.log(`修复图片URL: ${img.src}`);
        } catch (e) {
          console.warn(`无法修复图片URL: ${img.src}`, e);
        }
      }
    });

    // 预加载图片确保SingleFile捕获
    const preload = () => {
      document.querySelectorAll("img").forEach((img) => {
        if (!img.complete || img.naturalHeight === 0) {
          const loader = new Image();
          loader.src = img.src;
          loader.addEventListener("load", () => {
            console.log(`预加载图片完成: ${img.src}`);
          });
        }
      });
    };
    setTimeout(preload, 500);
  }

  // 主逻辑：SingleFile捕获前预处理
  let hasExecuted = false;
  let globalTimeout;

  addEventListener("single-file-on-before-capture-request", async (event) => {
    if (hasExecuted) {
      dispatchEvent(new CustomEvent("single-file-on-before-capture-response"));
      return;
    }

    hasExecuted = true;
    event.preventDefault();

    // 全局超时保护
    globalTimeout = setTimeout(() => {
      console.warn(
        `全局执行超时（超过${CONFIG.MAX_WAIT_TIME}ms），强制继续捕获`
      );
      dispatchEvent(new CustomEvent("single-file-on-before-capture-response"));
    }, CONFIG.MAX_WAIT_TIME);

    try {
      console.log("=== 开始SingleFile预处理 ===");

      // 1. 显示隐藏内容 + 移除广告
      showHiddenElements();
      await new Promise((resolve) => setTimeout(resolve, 300));

      // 2. 滚动触发懒加载
      console.log("滚动到页面底部（触发懒加载）");
      await scrollToBottom();
      // await new Promise((resolve) => setTimeout(resolve, CONFIG.SCROLL_DELAY));

      // console.log("滚动回页面顶部");
      // await scrollToTop();
      // await new Promise((resolve) => setTimeout(resolve, CONFIG.SCROLL_DELAY));

      // 4. 最终等待
      console.log(`等待${CONFIG.WAIT_AFTER_EXECUTION}ms确保资源就绪`);
      await new Promise((resolve) =>
        setTimeout(resolve, CONFIG.WAIT_AFTER_EXECUTION)
      );

      // 3. 检测扩展状态并决定处理方式
      const { available, isDownloadEnabled } = await checkExtensionAndStatus();
      if (available && isDownloadEnabled) {
        console.log("=== 扩展可用且下载功能启用，开始下载流程 ===");
        await startDownloadAndWait();
        console.log("=== 下载流程全部完成 ===");
      } else {
        console.log("=== 扩展不可用或下载功能禁用，使用替代方案 ===");
        // useAlternativeImageProcessing();
      }

      console.log("开始添加图片属性...");
      await addOriginalSrcToImagesAsync(); // 等待执行完成

      console.log(`等待${CONFIG.WAIT_AFTER_EXECUTION}ms确保资源就绪`);
      await new Promise((resolve) =>
        setTimeout(resolve, CONFIG.WAIT_AFTER_EXECUTION)
      );


      // 通知SingleFile开始捕获
      console.log("=== 预处理完成，通知SingleFile捕获 ===");
      dispatchEvent(new CustomEvent("single-file-on-before-capture-response"));
    } catch (error) {
      console.error("=== 预处理过程出错 ===", error.message);
      dispatchEvent(new CustomEvent("single-file-on-before-capture-response"));
    } finally {
      clearTimeout(globalTimeout);
    }
  });
})();

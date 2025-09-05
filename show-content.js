// ==UserScript==
// @name         显示隐藏内容脚本（SingleFile修复版）
// @namespace    https://example.com
// @version      2.6
// @description  在SingleFile保存前显示隐藏内容，修复扩展API调用问题
// @author       您的名称
// @match        *://*/*
// @grant        none
// ==/UserScript==

(function () {
  if (window.self !== window.top) {
    return;
  }

  // 初始化SingleFile用户脚本
  dispatchEvent(new CustomEvent("single-file-user-script-init"));

  // 保存原始状态
  let originalDisplayState = {};

  // 配置参数
  const CONFIG = {
    WAIT_AFTER_EXECUTION: 2000,
    SCROLL_DELAY: 1000,
    SCROLL_BEHAVIOR: "smooth",
    MAX_WAIT_TIME: 20000,
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
      '[class*="hidden"]',
      '[style*="display: none"]',
    ],
  };

  // 检测扩展API可用性
  function checkExtensionAPI() {
    return new Promise((resolve) => {
      // 检查扩展API是否已定义且方法可用
      if (
        typeof window.LiteImageDownloader !== "undefined" &&
        typeof window.LiteImageDownloader.startDownload === "function"
      ) {
        console.log("扩展API已直接可用");
        resolve(true);
        return;
      }

      // 尝试通过消息通信检测扩展
      const checkTimeout = setTimeout(() => {
        console.log("扩展API检测超时");
        resolve(false);
      }, 1000);

      // 监听扩展响应
      const responseHandler = function (event) {
        if (event.source !== window) return;
        if (
          event.data.type &&
          event.data.type === "LITE_IMAGE_DOWNLOADER_RESPONSE"
        ) {
          clearTimeout(checkTimeout);
          window.removeEventListener("message", responseHandler);
          console.log("扩展API通过消息通信可用");
          resolve(true);
        }
      };

      window.addEventListener("message", responseHandler);

      // 发送检测消息
      window.postMessage(
        {
          type: "LITE_IMAGE_DOWNLOADER",
          id: "extension_check",
          action: "getStatus",
        },
        "*"
      );
    });
  }

  // 滚动到页面底部的函数
  function scrollToBottom() {
    window.scrollTo({
      top: document.body.scrollHeight,
      behavior: CONFIG.SCROLL_BEHAVIOR,
    });
  }

  // 滚动到页面顶部的函数
  function scrollToTop() {
    window.scrollTo({
      top: 0,
      behavior: CONFIG.SCROLL_BEHAVIOR,
    });
  }

  // 显示元素并保存原始状态
  function showHiddenElements() {
    console.log("显示隐藏内容...");

    // 清除广告元素
    CONFIG.AD_SELECTORS.forEach((selector) => {
      const elements = document.querySelectorAll(selector);
      elements.forEach((element) => {
        const id =
          element.id || "ad_element_" + Math.random().toString(36).substr(2, 9);
        originalDisplayState[id] = element.style.display;
        element.style.display = "none";
        console.log("隐藏广告元素:", selector);
      });
    });

    // 显示隐藏内容
    CONFIG.HIDDEN_SELECTORS.forEach((selector) => {
      const elements = document.querySelectorAll(selector);
      elements.forEach((element) => {
        const id =
          element.id || "element_" + Math.random().toString(36).substr(2, 9);
        originalDisplayState[id] = element.style.display;
        element.style.display = "block";
        console.log("显示隐藏元素:", selector);

        // 添加可视化指示器
        if (
          element.id === "TextContent" ||
          element.classList.contains("hidden-content")
        ) {
          element.style.border = "2px dashed #4CAF50";
          element.style.position = "relative";

          const indicator = document.createElement("div");
          indicator.innerHTML = "此内容由用户脚本显示";
          indicator.style.position = "absolute";
          indicator.style.top = "5px";
          indicator.style.right = "5px";
          indicator.style.background = "#4CAF50";
          indicator.style.color = "white";
          indicator.style.padding = "2px 5px";
          indicator.style.borderRadius = "3px";
          indicator.style.fontSize = "12px";
          indicator.style.zIndex = "1000";
          element.appendChild(indicator);
        }
      });
    });
    // 为 div#TextContent 下的 img 元素添加属性（值为 src 本身）
    const textContentImages = document.querySelectorAll("div#TextContent img");
    textContentImages.forEach((img) => {
      if (img.src) {
        // 添加 data-original-src 属性，值与 src 相同
        img.setAttribute("data-original-src", img.src);
        console.log(`为 div#TextContent 下的图片添加属性: ${img.src}`);
      }
    });
  }

  // 通过消息通信调用扩展功能
  function callExtensionViaMessage(options) {
    return new Promise((resolve, reject) => {
      const id =
        "msg_" + Date.now() + "_" + Math.random().toString(36).substr(2, 9);

      // 设置超时
      const timeout = setTimeout(() => {
        reject(new Error("扩展调用超时"));
      }, 10000);

      // 监听响应
      const responseHandler = function (event) {
        if (event.source !== window) return;
        if (
          event.data.type &&
          event.data.type === "LITE_IMAGE_DOWNLOADER_RESPONSE" &&
          event.data.id === id
        ) {
          clearTimeout(timeout);
          window.removeEventListener("message", responseHandler);

          if (event.data.success) {
            resolve(event.data.data);
          } else {
            reject(new Error(event.data.error || "扩展调用失败"));
          }
        }
      };

      window.addEventListener("message", responseHandler);

      // 发送消息
      window.postMessage(
        {
          type: "LITE_IMAGE_DOWNLOADER",
          id: id,
          action: "startDownload",
          data: options,
        },
        "*"
      );
    });
  }

  // 调用扩展功能（如果可用）
  async function callExtensionAfterScroll() {
    console.log("滚动完成，准备调用扩展...");

    // 检查扩展API是否可用
    const extensionAvailable = await checkExtensionAPI();

    if (extensionAvailable) {
      console.log("检测到扩展API，开始调用...");

      try {
        // 尝试直接调用扩展功能
        if (
          typeof window.LiteImageDownloader !== "undefined" &&
          typeof window.LiteImageDownloader.startDownload === "function"
        ) {
          const response = await window.LiteImageDownloader.startDownload({
            regexPattern: null,
            outputPath: null,
            sendToRust: false,
            waitTime: 1000,
          });
          console.log("扩展直接调用成功:", response);
        } else {
          // 通过消息通信调用扩展
          const response = await callExtensionViaMessage({
            regexPattern: null,
            outputPath: null,
            sendToRust: false,
            waitTime: 1000,
          });
          console.log("扩展消息调用成功:", response);
        }
      } catch (error) {
        console.warn("扩展调用失败:", error);
        useAlternativeImageProcessing();
      }
    } else {
      console.log("未检测到扩展API，使用替代方案");
      useAlternativeImageProcessing();
    }
  }

  // 扩展不可用时的替代方案
  function useAlternativeImageProcessing() {
    console.log("使用替代图片处理方案...");

    // 这里可以添加替代的图像处理逻辑
    // 例如：直接下载图片、修改图片URL等

    // 示例：将所有图片URL改为绝对路径
    const images = document.querySelectorAll("img");
    images.forEach((img) => {
      if (img.src && !img.src.startsWith("http")) {
        try {
          img.src = new URL(img.src, window.location.href).href;
          console.log("修复图片URL:", img.src);
        } catch (e) {
          console.warn("无法修复图片URL:", img.src, e);
        }
      }
    });

    // 示例：预加载图片以确保SingleFile能捕获它们
    const preloadImages = () => {
      const images = document.querySelectorAll("img");
      images.forEach((img) => {
        if (img.complete && img.naturalHeight !== 0) return;

        const loader = new Image();
        loader.src = img.src;
        console.log("预加载图片:", img.src);
      });
    };

    // 延迟执行预加载，确保页面已稳定
    setTimeout(preloadImages, 500);
  }

  let hasExecuted = false;
  let executionTimeout;

  addEventListener("single-file-on-before-capture-request", async (event) => {
    if (hasExecuted) {
      dispatchEvent(new CustomEvent("single-file-on-before-capture-response"));
      return;
    }

    hasExecuted = true;
    event.preventDefault();

    // 设置超时保护
    executionTimeout = setTimeout(() => {
      console.warn("用户脚本执行超时，强制继续");
      dispatchEvent(new CustomEvent("single-file-on-before-capture-response"));
    }, CONFIG.MAX_WAIT_TIME);

    try {
      console.log("开始显示隐藏内容...");

      // 1. 显示隐藏元素
      showHiddenElements();

      // 等待元素显示
      await new Promise((resolve) => setTimeout(resolve, 100));

      // 2. 执行向下滚动操作
      console.log("滚动到页面底部...");
      scrollToBottom();
      // 等待滚动完成
      await new Promise((resolve) => setTimeout(resolve, CONFIG.SCROLL_DELAY));

      // 3. 滚动回顶部
      console.log("滚动回页面顶部...");
      scrollToTop();
      await new Promise((resolve) => setTimeout(resolve, CONFIG.SCROLL_DELAY));

      // 4. 滚动完成后调用扩展（如果可用）
      await callExtensionAfterScroll();

      // 5. 执行完后的额外等待时间
      console.log(`等待 ${CONFIG.WAIT_AFTER_EXECUTION}ms 后再继续...`);
      await new Promise((resolve) =>
        setTimeout(resolve, CONFIG.WAIT_AFTER_EXECUTION)
      );

      dispatchEvent(new CustomEvent("single-file-on-before-capture-response"));
    } catch (error) {
      console.error("SingleFile脚本执行错误:", error);
      dispatchEvent(new CustomEvent("single-file-on-before-capture-response"));
    } finally {
      clearTimeout(executionTimeout);
      dispatchEvent(new CustomEvent("single-file-on-before-capture-response"));
    }
  });
})();

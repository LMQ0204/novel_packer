// inject.js - 注入到网页中，为用户脚本提供API
(function() {
  // 生成唯一ID
  function generateId() {
    return Date.now().toString(36) + Math.random().toString(36).substr(2);
  }

  // 存储回调函数
  const callbacks = {};
  // 新增：存储当前下载功能是否启用的状态
  let isDownloadEnabled = true;

  // 新增：初始化时获取一次状态（获取 openDownload 配置）
  function initStatus() {
    return new Promise((resolve) => {
      const id = generateId();
      callbacks[id] = {
        resolve: (data) => {
          isDownloadEnabled = data?.isDownloadEnabled ?? true;
          resolve();
        },
        reject: () => resolve(), // 失败时默认认为启用
      };

      window.postMessage(
        {
          type: "LITE_IMAGE_DOWNLOADER",
          id: id,
          action: "getStatus",
          data: {},
        },
        "*"
      );
    });
  }

  // 监听扩展的响应
  window.addEventListener("message", function (event) {
    if (event.source !== window) return;

    if (
      event.data.type &&
      event.data.type === "LITE_IMAGE_DOWNLOADER_RESPONSE"
    ) {
      const { id, success, data, error } = event.data;

      if (callbacks[id]) {
        if (success) {
          // 若为getStatus响应，实时更新启用状态
          if (data?.isDownloadEnabled !== undefined) {
            isDownloadEnabled = data.isDownloadEnabled;
          }
          callbacks[id].resolve(data);
        } else {
          callbacks[id].reject(error);
        }
        delete callbacks[id];
      }
    }

    if (event.data.type && event.data.type === "LITE_IMAGE_DOWNLOADER_EVENT") {
      // 触发事件
      if (window.LiteImageDownloader && window.LiteImageDownloader.events) {
        const eventName = event.data.event;
        const eventData = event.data.data;

        if (window.LiteImageDownloader.events[eventName]) {
          window.LiteImageDownloader.events[eventName].forEach((callback) => {
            try {
              callback(eventData);
            } catch (e) {
              console.error("Error in event handler:", e);
            }
          });
        }
      }
    }
  });

  // 初始化时获取状态
  initStatus();

  // 创建全局API对象
  window.LiteImageDownloader = {
    // 配置
    config: {
      regexPattern: null,
      outputPath: null,
      sendToRust: false,
      waitTime: 1000,
    },

    // 新增：暴露下载功能是否启用的状态
    isDownloadEnabled: () => isDownloadEnabled,

    // 事件系统
    events: {},

    // 发送消息到扩展
    sendMessage: function (message) {
      // 若功能禁用，直接拒绝与下载相关的操作
      if (!isDownloadEnabled && message.action === "startDownload") {
        return Promise.reject("Download function is disabled by configuration");
      }
      return new Promise((resolve, reject) => {
        const id = generateId();
        callbacks[id] = { resolve, reject };

        window.postMessage(
          {
            type: "LITE_IMAGE_DOWNLOADER",
            id: id,
            action: message.action,
            data: message.data,
          },
          "*"
        );
      });
    },

    // 开始下载
    startDownload: function (options = {}) {
      if (!isDownloadEnabled) {
        return Promise.reject(
          new Error("Download function is disabled by configuration")
        );
      }
      // 合并配置
      const config = { ...this.config, ...options };
      return this.sendMessage({
        action: "startDownload",
        data: config,
      });
    },

    // 停止下载
    stopDownload: function () {
      return this.sendMessage({
        action: "stopDownload",
      });
    },

    // 获取状态
    getStatus: function () {
      return this.sendMessage({
        action: "getStatus",
      });
    },

    // 新增：强制刷新状态（可选，供页面主动查询）
    refreshStatus: function () {
      return initStatus();
    },

    // 获取并更新最新配置
    fetchLatestConfig: function () {
      return this.sendMessage({
        action: "fetchLatestConfig",
      });
    },

    // 注册事件监听器
    on: function (eventName, callback) {
      if (!this.events[eventName]) {
        this.events[eventName] = [];
      }
      this.events[eventName].push(callback);
    },

    // 移除事件监听器
    off: function (eventName, callback) {
      if (this.events[eventName]) {
        const index = this.events[eventName].indexOf(callback);
        if (index > -1) {
          this.events[eventName].splice(index, 1);
        }
      }
    },
  };

  // 定义事件类型
  window.LiteImageDownloader.EVENTS = {
    DOWNLOAD_STARTED: "downloadStarted",
    DOWNLOAD_PROGRESS: "downloadProgress",
    DOWNLOAD_COMPLETED: "downloadCompleted",
    DOWNLOAD_ERROR: "downloadError",
    CONFIG_UPDATED: "configUpdated",
  };

  console.log("Lite Image Downloader API loaded");
  // 新增：在控制台提示当前功能状态
  console.log(`Download function is ${isDownloadEnabled ? 'enabled' : 'disabled'}`);
  console.log(
    "[Inject 调试] API 挂载到 window 结果：",
    window.LiteImageDownloader
  );
  console.log("[Inject 调试] window 对象是否存在：", !!window);
})();
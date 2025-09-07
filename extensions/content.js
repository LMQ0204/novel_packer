// content.js - 内容脚本，作为网页和扩展之间的桥梁

// 监听来自网页的消息
window.addEventListener('message', function(event) {
  // 只处理来自同一源的消息，并检查消息结构
  if (event.source !== window) return;
  
  if (event.data.type && event.data.type === 'LITE_IMAGE_DOWNLOADER') {
    // 将消息转发到扩展
    chrome.runtime.sendMessage(event.data, function(response) {
      // 将响应发送回网页
      window.postMessage({
        type: 'LITE_IMAGE_DOWNLOADER_RESPONSE',
        id: event.data.id,
        success: response.success,
        data: response.data,
        error: response.error
      }, '*');
    });
  }
});

// 监听来自扩展的消息并将其转发到网页
chrome.runtime.onMessage.addListener(function(request, sender, sendResponse) {
  if (request.type && request.type === 'LITE_IMAGE_DOWNLOADER_EVENT') {
    window.postMessage(request, '*');
  }
  return true;
});

// 注入脚本到网页中，使API对用户脚本可用
const script = document.createElement('script');
script.src = chrome.runtime.getURL('inject.js');
(document.head || document.documentElement).appendChild(script);
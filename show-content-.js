// ==UserScript==
// @name         显示隐藏内容脚本（带滚动和延迟）
// @namespace    https://example.com
// @version      1.2
// @description  在SingleFile保存前显示隐藏的TextContent div，滚动页面并支持延迟
// @author       Your Name
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
    const WAIT_AFTER_EXECUTION = 3000; // 执行后等待时间（毫秒）
    const SCROLL_DELAY = 1000; // 滚动后等待时间（毫秒）
    
    // 滚动到页面底部的函数
    function scrollToBottom() {
        window.scrollTo({
            top: document.body.scrollHeight,
            behavior: 'smooth' // 平滑滚动
        });
    }
    
    // 滚动到页面顶部的函数（可选）
    function scrollToTop() {
        window.scrollTo({
            top: 0,
            behavior: 'smooth' // 平滑滚动
        });
    }

    let hasExecuted = false;
    let executionTimeout;
    
    
    addEventListener("single-file-on-before-capture-request", async event => {
        if (hasExecuted) {
            dispatchEvent(new CustomEvent("single-file-on-before-capture-response"));
            return;
        }
        
        hasExecuted = true;
        event.preventDefault();

        // 设置超时保护（最多等待10秒）
        executionTimeout = setTimeout(() => {
            console.warn("用户脚本执行超时，强制继续");
            dispatchEvent(new CustomEvent("single-file-on-before-capture-response"));
        }, 20000);
        
        try {
            console.log(" 开始清除广告...");
            // 2. 处理广告元素：查找并隐藏/移除
            const adElements = document.querySelectorAll('ins.adsbygoogle');
            adElements.forEach(element => {
                // 可以选择直接移除元素，或者设置display: none
                // 移除元素：element.remove(); 
                element.remove();
                // 隐藏元素（记录原始状态，不过广告元素可能本就不需要恢复，看需求）
                // const id = element.id || 'ad_element_' + Math.random().toString(36).substr(2, 9);
                // originalDisplayState[id] = element.style.display;
                // element.style.display = 'none'; 
            });

            // 查找所有需要显示的隐藏元素
            const hiddenElements = document.querySelectorAll('#mlfy_main_text #TextContent, #mlfy_main_text [style*="display: none"], #mlfy_main_text [style*="display:none"]');
            
            // 保存原始状态并显示元素
            hiddenElements.forEach(element => {
                const id = element.id || 'element_' + Math.random().toString(36).substr(2, 9);
                originalDisplayState[id] = element.style.display;
                element.style.display = 'block';
            });
            
            // 特别处理TextContent元素
            const textContent = document.getElementById('TextContent');
            if (textContent) {
                originalDisplayState.TextContent = textContent.style.display;
                textContent.style.display = 'block';
                
                // 添加可视化指示器
                textContent.style.border = '2px dashed #4CAF50';
                textContent.style.position = 'relative';
                
                const indicator = document.createElement('div');
                indicator.innerHTML = '此内容由用户脚本显示';
                indicator.style.position = 'absolute';
                indicator.style.top = '5px';
                indicator.style.right = '5px';
                indicator.style.background = '#4CAF50';
                indicator.style.color = 'white';
                indicator.style.padding = '2px 5px';
                indicator.style.borderRadius = '3px';
                indicator.style.fontSize = '12px';
                textContent.appendChild(indicator);
            }
            
            // 等待元素显示
            await new Promise(resolve => setTimeout(resolve, 100));
            
            // 执行向下滚动操作
            console.log("滚动到页面底部...");
            scrollToBottom();
            
            // 等待滚动完成
            await new Promise(resolve => setTimeout(resolve, SCROLL_DELAY));
            
            // （可选）如果需要，可以添加滚动回顶部的操作
            console.log("滚动回页面顶部...");
            scrollToTop();
            await new Promise(resolve => setTimeout(resolve, SCROLL_DELAY));
            
            // 执行完后的额外等待时间
            console.log(`将等待 ${WAIT_AFTER_EXECUTION}ms 后再继续...`);
            await new Promise(resolve => setTimeout(resolve, WAIT_AFTER_EXECUTION));

            dispatchEvent(new CustomEvent("single-file-on-before-capture-response"));
        } catch (error) {
            console.error('SingleFile脚本执行错误:', error);
            dispatchEvent(new CustomEvent("single-file-on-before-capture-response"));
        } finally {
            clearTimeout(executionTimeout);
            
            // 通知SingleFile异步操作完成
            dispatchEvent(new CustomEvent("single-file-on-before-capture-response"));
        }
    });
})();

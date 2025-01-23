const config = window.protectConfig || {};

// 是否开启所有功能的开关
const enableAllProtections = config.enableAllProtections !== undefined ? config.enableAllProtections : false;

// 如果 enableAllProtections 为 true，则将所有其他选项设置为 true
const disableMobileSupport = enableAllProtections || (config.disableMobileSupport !== undefined ? config.disableMobileSupport : true);
const enableKeyboardProtection = enableAllProtections || (config.enableKeyboardProtection !== undefined ? config.enableKeyboardProtection : false);
const enableFocusProtection = enableAllProtections || (config.enableFocusProtection !== undefined ? config.enableFocusProtection : false);
const enableRightClickProtection = enableAllProtections || (config.enableRightClickProtection !== undefined ? config.enableRightClickProtection : false);
const enableCopyProtection = enableAllProtections || (config.enableCopyProtection !== undefined ? config.enableCopyProtection : false);
const enableDevToolsProtection = enableAllProtections || (config.enableDevToolsProtection !== undefined ? config.enableDevToolsProtection : false);
const enablePrintProtection = enableAllProtections || (config.enablePrintProtection !== undefined ? config.enablePrintProtection : false);
const enableScreenshotProtection = enableAllProtections || (config.enableScreenshotProtection !== undefined ? config.enableScreenshotProtection : false);
const enableInfiniteDebugger = enableAllProtections || (config.enableInfiniteDebugger !== undefined ? config.enableInfiniteDebugger : false);
// 推断总开关的值
const isProtectionEnabled = enableKeyboardProtection || enableFocusProtection || enableRightClickProtection || enableCopyProtection || enableDevToolsProtection || enablePrintProtection || enableScreenshotProtection || enableInfiniteDebugger;

// 如果总开关为 false，则终止脚本执行
if (!isProtectionEnabled) {
    console.log('所有保护功能已关闭，脚本终止执行。');
    throw new Error('所有保护功能已关闭，脚本终止执行。');
}

// 检测是否为移动设备
function isMobileDevice() {
    return 'ontouchstart' in window || navigator.maxTouchPoints > 0 || navigator.msMaxTouchPoints > 0;
}

// 检测是否为QQ、微信、阿里内置浏览器
function isNeizhi() {
    const ua = navigator.userAgent.toLowerCase();
    if (ua.match(/MicroMessenger/i) === "micromessenger") {
        return "weixin";
    } else if (ua.match(/QQ/i) === "qq") {
        return "QQ";
    } else if (ua.match(/Alipay/i) === "alipay") {
        return "alipay";
    }
    return false;
}

// 禁用移动设备支持
function disableMobileDeviceSupport() {
    if (enableScreenshotProtection && isMobileDevice() && disableMobileSupport) {
        // 立即清空页面内容，防止泄露
        document.body.innerHTML = '';
        // 检测是否为QQ、微信、阿里内置浏览器
        const neizhiBrowser = isNeizhi();
        if (neizhiBrowser) {

            // 提示用户使用外部浏览器打开
            const warningMessage = document.createElement('div');
            warningMessage.style.textAlign = 'center';
            warningMessage.style.fontSize = '26px';
            warningMessage.style.color = 'red';
            warningMessage.innerHTML = `
                ⚠️<br>
                抱歉，本页面不支持在${neizhiBrowser}内置浏览器中访问。<br>
                请使用外部浏览器打开以继续浏览。<br>
            `;

            // 设置 body 为 flex 容器，并使内容居中
            document.body.style.display = 'flex';
            document.body.style.justifyContent = 'center';
            document.body.style.alignItems = 'flex-start';
            document.body.style.height = '40%';
            document.body.style.margin = '0';
            document.body.style.paddingTop = '60%';

            // 将警告信息添加到页面中
            document.body.appendChild(warningMessage);

            // 停止后续脚本执行
            throw new Error('Mobile device support is disabled.');
        }

        // 尝试跳转到 safebrowser 应用
        const url = encodeURIComponent(window.location.href);
        const safebrowserUrl = `safebrowser://open?url=${url}`;

        // 创建跳转链接
        const link = document.createElement('a');
        link.href = safebrowserUrl;
        link.style.display = 'none';
        document.body.appendChild(link);

        // 尝试跳转
        link.click();

        // 设置一个超时，检查是否跳转成功
        setTimeout(() => {
            // 如果跳转失败，提示用户下载 APK
            const warningMessage = document.createElement('div');
            warningMessage.style.textAlign = 'center';
            warningMessage.style.fontSize = '26px';
            warningMessage.style.color = 'red';
            warningMessage.innerHTML = `
                ⚠️<br>
                本页面不允许截屏、录屏、复制。<br>
                请使用指定阅读器或PC浏览器打开：<br>
                <a href="/static/blog-reader.apk" style="color: blue; text-decoration: underline;">点击下载 Blog Reader</a>
            `;

            // 设置 body 为 flex 容器，并使内容居中
            document.body.style.display = 'flex';
            document.body.style.justifyContent = 'center';
            document.body.style.alignItems = 'flex-start';
            document.body.style.height = '40%';
            document.body.style.margin = '0';
            document.body.style.paddingTop = '60%';

            // 将警告信息添加到页面中
            document.body.appendChild(warningMessage);

            // 停止后续脚本执行
            throw new Error('Mobile device support is disabled.');
        }, 1000); // 1秒后检查是否跳转成功
    }
}

// 在页面加载时检测并禁用移动设备支持
disableMobileDeviceSupport();


// 全局变量
let originalContent = ''; // 保存原始内容
let isProtected = false; // 是否处于保护状态
let protectTimer = null; // 保护状态计时器
let isMouseInWindow = true; // 光标是否在页面范围内
let isPageVisible = true; // 页面是否可见
let isFocused = true; // 页面是否获得焦点

// 获取当前页面的URL
const currentUrl = window.location.href;

// 判断当前URL是否以index.html结尾
let baseUrl = currentUrl;
if (!currentUrl.endsWith('index.html')) {
    // 如果当前URL不以index.html结尾，则添加index.html
    baseUrl = currentUrl.endsWith('/') ? `${currentUrl}index.html` : `${currentUrl}/index.html`;
}

// 拼接加密内容的URL
const encryptedContentUrl = `${baseUrl}.encrypted`;

// 获取<article>元素
const articleElement = document.querySelector('article');

// 创建模糊遮罩和警告框
const overlay = document.createElement('div');
overlay.style.position = 'fixed';  // 改为 fixed
overlay.style.top = '0';
overlay.style.left = '0';
overlay.style.width = '100%';
overlay.style.height = '100%';
overlay.style.backdropFilter = 'blur(10px)';
overlay.style.backgroundColor = 'rgba(255, 255, 255, 0.5)';
overlay.style.display = 'none';
overlay.style.zIndex = '1000';
overlay.style.justifyContent = 'center';
overlay.style.alignItems = 'center';

const warningBox = document.createElement('div');
warningBox.style.background = '#ffcccc';
warningBox.style.padding = '20px';
warningBox.style.borderRadius = '10px';
warningBox.style.boxShadow = '0 0 10px rgba(0, 0, 0, 0.1)';
warningBox.innerHTML = `
    <h1>检测到非法操作</h1>
    <p>页面内容已隐藏，请勿尝试截屏或其他非法操作。</p>
`;

overlay.appendChild(warningBox);
document.body.appendChild(overlay);

// 100个常用汉字
const commonChineseCharacters = [
    '的', '一', '是', '在', '不', '了', '有', '和', '人', '这',
    '中', '大', '为', '上', '个', '国', '我', '以', '要', '他',
    '时', '来', '用', '们', '生', '到', '作', '地', '于', '出',
    '就', '分', '对', '成', '会', '可', '主', '发', '年', '动',
    '同', '工', '也', '能', '下', '过', '子', '说', '产', '种',
    '面', '而', '方', '后', '多', '定', '行', '学', '法', '所',
    '民', '得', '经', '十', '三', '之', '进', '着', '等', '部',
    '度', '家', '电', '力', '里', '如', '水', '化', '高', '自',
    '二', '理', '起', '小', '物', '现', '实', '加', '量', '都',
    '两', '体', '制', '机', '当', '使', '点', '从', '业', '本'
];

// 常用标点符号
const punctuationMarks = ['，', '。', '；', '！', '？'];

// 生成随机常用汉字，并在稍短随机间隔插入标点
function generateRandomCommonChinese(length, minInterval = 5, maxInterval = 10, minParagraphLength = 100, maxParagraphLength = 200) {
    let result = '';
    let count = 0; // 记录当前连续汉字的长度
    let totalLength = 0; // 记录总长度

    while (totalLength < length) {
        // 随机生成段落长度
        const paragraphLength = Math.floor(Math.random() * (maxParagraphLength - minParagraphLength + 1)) + minParagraphLength;
        let paragraph = '<p>';
        let currentParagraphLength = 0;

        while (currentParagraphLength < paragraphLength && totalLength < length) {
            // 随机选择一个常用汉字
            const randomIndex = Math.floor(Math.random() * commonChineseCharacters.length);
            paragraph += commonChineseCharacters[randomIndex];
            count++;
            currentParagraphLength++;
            totalLength++;

            // 随机决定是否插入标点
            if (count >= minInterval && (count >= maxInterval || Math.random() < 0.3)) {
                const randomPunctuation = punctuationMarks[Math.floor(Math.random() * punctuationMarks.length)];
                paragraph += randomPunctuation;
                count = 0; // 重置计数器
            }
        }

        paragraph += '</p>';
        result += paragraph;
    }

    return result;
}

// 获取加密内容并解密
async function loadAndDecryptContent() {
    try {
        const response = await fetch(encryptedContentUrl);
        const encryptedContent = await response.text();

        // 检查获取到的内容是否为空
        if (!encryptedContent) {
            throw new Error('获取到的加密内容为空');
        }

        const decodedContent = base64DecodeUtf8(encryptedContent); // 使用自定义的 Base64 解码函数
        originalContent = decodedContent;
        articleElement.innerHTML = decodedContent;
    } catch (error) {
        console.error('Failed to load or decrypt content:', error);
        originalContent = articleElement.innerHTML;
    }
}

// 自定义 Base64 解码函数，支持 UTF-8 编码
function base64DecodeUtf8(str) {
    // 将 Base64 字符串转换为字节数组
    const binaryString = atob(str);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
    }
    // 使用 TextDecoder 将字节数组解码为 UTF-8 字符串
    const decoder = new TextDecoder('utf-8');
    return decoder.decode(bytes);
}

// 保护内容并显示提示
function protectContent() {
    if (!isProtected) {
        // 替换内容为等字数的随机汉字
        const placeholder = generateRandomCommonChinese(800);
        articleElement.innerHTML = placeholder;

        // 显示模糊遮罩和警告框
        overlay.style.display = 'flex';
        isProtected = true;

        // 2秒内无法解除保护
        clearTimeout(protectTimer);
        protectTimer = setTimeout(() => {
            protectTimer = null
            unprotectContent();
        }, 1300);
    }
}

// 取消保护内容并隐藏提示
function unprotectContent() {
    if (isProtected && !protectTimer) {
        // 只有当光标在页面、页面可见且获得焦点时，才解除保护
        if (isMouseInWindow && isPageVisible && isFocused) {
            articleElement.innerHTML = originalContent;
            overlay.style.display = 'none';
            isProtected = false;
        }
    }
}

// 监听键盘事件
if (enableKeyboardProtection) {
    document.addEventListener('keydown', (event) => {
        if (!isProtected) {
            protectContent();
        }
    });
}

// 监听鼠标移入页面事件
if (enableFocusProtection) {
    document.addEventListener('mouseenter', () => {
        isMouseInWindow = true;
        unprotectContent();
    });

    // 监听鼠标移出页面事件
    document.addEventListener('mouseleave', () => {
        isMouseInWindow = false;
        protectContent();
    });

    // 监听页面可见性变化事件
    document.addEventListener('visibilitychange', () => {
        isPageVisible = !document.hidden;
        if (isPageVisible) {
            unprotectContent();
        } else {
            protectContent();
        }
    });

    // 监听页面获得焦点事件
    window.addEventListener('focus', () => {
        isFocused = true;
        unprotectContent();
    });

    // 监听页面失去焦点事件
    window.addEventListener('blur', () => {
        isFocused = false;
        protectContent();
    });
}

// 禁用右键菜单
if (enableRightClickProtection) {
    document.addEventListener('contextmenu', (event) => {
        event.preventDefault();
        console.log('右键菜单被禁用');
    });
}

// 防止拖拽和选择文本（整合到防复制功能中）
if (enableCopyProtection) {
    document.addEventListener('dragstart', (event) => {
        event.preventDefault();
        console.log('拖拽被阻止');
    });

    document.addEventListener('selectstart', (event) => {
        event.preventDefault();
        console.log('文本选择被阻止');
    });

    document.body.oncopy = () => false;
    document.body.oncontextmenu = () => false;
    document.body.onselectstart = document.body.ondrag = () => false;
    document.onkeydown = (event) => {
        if ((event.ctrlKey || event.metaKey) && event.key === 's') {
            event.preventDefault();
        }
    };
}

// 防止开发者工具（F12 和 Ctrl + Shift + I）
if (enableDevToolsProtection) {
    document.addEventListener('keydown', (event) => {
        if (event.key === 'F12' || (event.ctrlKey && event.shiftKey && event.key === 'I')) {
            event.preventDefault();
            console.log('开发者工具被阻止');
        }
    });
}

// 防止打印
if (enablePrintProtection) {
    const cssNode = document.createElement('style');
    cssNode.media = 'print';
    cssNode.innerHTML = 'body { display: none; }';
    document.head.appendChild(cssNode);

    document.addEventListener('keydown', (event) => {
        if (event.ctrlKey && event.key === 'p') {
            event.stopPropagation();
            event.preventDefault();
            event.stopImmediatePropagation();
        }
    });
}

// 防止截屏
if (enableScreenshotProtection) {
    document.addEventListener('keyup', (event) => {
        if (event.key === 'PrintScreen') {
            navigator.clipboard.writeText('');
        }
    });
}

// 无限 debugger
if (enableInfiniteDebugger) {
    setInterval(() => {
        debugger;
    }, 100);
}

// 初始化：加载并解密内容
loadAndDecryptContent();
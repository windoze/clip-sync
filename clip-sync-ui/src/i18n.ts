import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';

i18n
    // 检测用户当前使用的语言
    // 文档: https://github.com/i18next/i18next-browser-languageDetector
    .use(LanguageDetector)
    // 注入 react-i18next 实例
    .use(initReactI18next)
    // 初始化 i18next
    // 配置参数的文档: https://www.i18next.com/overview/configuration-options
    .init({
        debug: true,
        fallbackLng: 'en',
        interpolation: {
            escapeValue: false,
        },
        // lng: 'zh',
        resources: {
            en: {
                translation: {
                    // 这里是我们的翻译文本
                    labelText: 'Text',
                    labelImage: 'Image',
                    labelUtils: 'Utilities',
                    searchPlaceholder: 'Search text',
                    resultCount: 'Found {{count}} result',
                    deviceListPlaceholder: 'Copied from...',
                    timeRangeStart: 'Start time',
                    timeRangeEnd: 'End time',
                    copyTextToClipboard: "Copy text to clipboard",
                    copiedMessage: "Copied",
                    inputText: "Input text",
                    sendText: "Send",
                    copyTextMessage: "{{source}} copied text '{{text}}'",
                    copyImageMessage: "{{source}} copied image",
                    systemUtil: "System",
                    unsupportedEntry: "Unsupported clipboard format",
                }
            },
            zh: {
                translation: {
                    labelText: '文本',
                    labelImage: '图片',
                    labelUtils: '工具',
                    searchPlaceholder: '搜索文本',
                    resultCount: '找到 {{count}} 个结果',
                    deviceListPlaceholder: '复制来源...',
                    timeRangeStart: '起始时间',
                    timeRangeEnd: '结束时间',
                    copyTextToClipboard: "复制文本到剪贴板",
                    copiedMessage: "已复制",
                    inputText: "输入文本",
                    sendText: "发送",
                    copyTextMessage: "{{source}} 复制了文本 '{{text}}'",
                    copyImageMessage: "{{source}} 复制了图片",
                    systemUtil: "系统工具",
                    unsupportedEntry: "不支持的剪贴板格式",
                }
            }
        }
    });

export default i18n;

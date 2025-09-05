pub const BROKEN_IMAGE_BASE64: &str = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48cmVjdCB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgZmlsbD0iI2ZmZiIvPjxjaXJjbGUgY3g9IjUwIiBjeT0iNTAiIHI9IjQwIiBmaWxsPSIjNzc3Ii8+PHBhdGggZD0iTTMwIDMwIEw3MCA3MCIgc3Ryb2tlPSIjMzMzIiBzdHJva2Utd2lkdGg9IjUiLz48cGF0aCBkPSJNMzAgNzAgTDcwIDMwIiBzdHJva2U9IiMzMzMiIHN0cm9rZS13aWR0aD0iNSIvPjwvc3ZnPg==";
pub const DEFAULT_CSS: &str = r#"
    body {
        font-family: "Noto Serif", "SimSun", serif;
        font-size: 1em;
        line-height: 1.8;
        margin: 0;
        padding: 1em;
        text-align: justify;
        color: #333;
        background-color: #fff;
    }
    h1 {
        font-size: 1.3em;
        text-align: center;
        margin: 1.5em 0 1em;
        padding-bottom: 0.5em;
        border-bottom: 1px solid #e0e0e0;
        font-weight: bold;
        color: #222;
    }
    .toc-title {
        text-align: center;
        font-size: 1.5em;
        margin-bottom: 1.2em;
        font-weight: bold;
        color: #222;
    }
    .toc-list {
        list-style-type: none;
        padding: 0;
        margin: 0;
    }
    .toc-item {
        margin: 0.6em 0;
        border-bottom: 1px dashed #e0e0e0;
    }
    .toc-link {
        text-decoration: none;
        color: #1a5fb4;
        display: block;
        padding: 0.4em 0.8em;
        transition: background-color 0.2s ease;
        font-size: 0.95em;
    }
    .toc-link:hover {
        background-color: #f8f8f8;
        color: #00468b;
    }
    img {
        max-width: 95%;
        height: auto;
        display: block;
        margin: 1em auto;
        border-radius: 3px;
        box-shadow: 0 2px 5px rgba(0,0,0,0.1);
    }
    p {
        margin: 1.2em 0;
        text-indent: 2em;
        text-align: justify;
        hyphens: auto;
    }
    .cover-image {
        display: block;
        max-width: 80%;
        height: auto;
        margin: 2em auto;
        border-radius: 5px;
        box-shadow: 0 4px 10px rgba(0,0,0,0.15);
    }
    @media (prefers-color-scheme: dark) {
        body {
            color: #ddd;
            background-color: #1a1a1a;
        }
        h1, .toc-title {
            color: #eee;
            border-bottom-color: #444;
        }
        .toc-item {
            border-bottom-color: #444;
        }
        .toc-link {
            color: #4eb2ff;
        }
        .toc-link:hover {
            background-color: #2a2a2a;
            color: #7ac2ff;
        }
    }
"#;
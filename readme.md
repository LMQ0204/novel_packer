<h1 align="center">小说打包器</h1>
<hr/>

## 介绍

本人的学习项目，有许多不足。由于本人并不是很了解爬虫以及相关的知识，所以参考且使用了其他仓库的工具或代码。

轻小说打包器，可以将支持的轻小说网站中的小说打包成EPUB格式，包含插图，并自动生成目录页。
这个工具本质上是控制浏览器自动化进行操作，所以可能内存占用会比较高。

使用前先确保默认文件夹中安装了 Chrome 或基于 Chromium 的浏览器。否则你需要在`./config/browser.json`和`./config/browser_check.json`中指定你的浏览器路径。
### 目前支持的小说网站
 - [哔哩轻小说](https://www.linovelib.com)


## 下载

下载此项目并解压。
```
git clone https://github.com/LMQ0204/novel_packer.git
```

## 使用
双击exe打开，或者使用命令提示符打开即可。然后根据提示进行操作。

![01](./images/novel.png)

还支持中断下载或者检查过程并保存状态，然后下一次可以从文件恢复保存的状态。

![02](./images/恢复下载.png)

支持自定义`epub`的`css`样式。我推荐将`css`文件放在`./assets`里面，并在`./config/bilinovel.json`的`"css"`中指明它的路径。

### 基础配置
`config`文件夹下存储着运行的相关配置。我不建议你去随意更改，除非你真的知道这些配置的作用。但是有几个选项可以根据个人意愿稍作修改。

`config/bilinovel.json`里面的`max_concurrent`表示章节下载的最大并发数，使用这个可以控制同时下载的最大章节数。增大这个值可以在一定程度上提升下载速度，但是这会增加被限制的风险，以及使用内存增加。

`config/bilinovel.json`里面的`check_rounds`表示下载完成后，检查图片操作的最大次数。

`config/bilinovel.json`里面的`css`表示的是打包`epub`时,使用的css文件的路径(可以使用相对路径)。

### 高级配置

除了上述配置外，还有比较复杂的配置。

`config/novel.json`、`config/chaoter.json`、`config/images.json`分别储存着下载小说页面、章节页面以及重新下载缺少图片或缺页的页面时，所使用的`single-file`命令行工具的配置，最好不要随意改动，详见(https://github.com/gildas-lormeau/single-file-cli)。

`config/browser.json`存储的是打开浏览器实例时的命令行选项。

`config/http.json`存储的是rust服务器的相关配置，主要作用是接受浏览器扩展上传的图片数据。其中`regex_pattern`用来筛选图片的url，匹配的图片会被保留。`open_download`表示使用开启扩展的图片下载功能。`server_port`表示服务器开启的端口号。`send_to_rust`表示是否将图片数据上传到本地服务器。`wait_time`表示下载间隔。`save_to_file`表示是否将图片保存到本地。`output_path`表示图片的保存路径，这个选项是在`save_to_file`为`true`时有用。

### 其他

`extensions`下面保存的是用于保存图片的浏览器插件(根据[lite-image-downloader](https://github.com/belaviyo/lite-image-downloader)修改)。

`extra`下面是[single-file-cil](https://github.com/gildas-lormeau/single-file-cli)的二进制文件。

`output`中是打包好的`epub`

`scripts`下面是用户脚本。

`temp`下面是下载进度文件、日志、临时文件等。其中`./temp/download`下面是下载的文本文件，`./temp/images`下面是压缩成二进制格式的图片。如果你担心下载进度丢失的话，以上两个文件夹需要谨慎清理。而`./temp/logs`下存放的是日志文件,`./temp/temp`下是临时文件，每次下载开始或结束都可以清理。

`user data`是浏览器配置文件。如果你想要放在其他位置，相应地`./config/browser.json`和`./config/browser_check.json`中也需要更改。

## 常见问题

### 速度问题

下载速度极大程度上取决于页面加载速度。

### 内存占用问题

该工具本质上是自动开启一个浏览器，然后打开页面进行下载。`max_concurrent`的值会直接决定打开的页面数量，对于内存影响很大。

### 修改小说

解压`epub`进行修改。

## 编译

目前只支持Windows

### windows
执行目录下的[**build.bat**](./build.bat)即可。

或者手动执行下列命令，并将生成的`exe`复制到当前目录下。
```
cargo build --release
```

## 贡献

我在项目中留下了许多未使用的代码或函数，其目的是希望其他人能够方便地为项目作贡献及修改。

## 依赖及参考的代码或工具

[single-file-cil](https://github.com/gildas-lormeau/single-file-cli)：大多数下载是在这个工具的基础上进行的。

[lite-image-downloader](https://github.com/belaviyo/lite-image-downloader)：下载图片的功能是基于这个进行修改，然后将图片传输到本地服务器。

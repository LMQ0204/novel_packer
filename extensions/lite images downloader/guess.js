const MIME_TYPES = {
  'image/jpeg': 'jpg',
  'application/x-javascript': 'js',
  'application/atom+xml': 'atom',
  'application/rss+xml': 'rss',
  'text/plain': 'txt',
  'text/javascript': 'js',
  'image/x-icon': 'ico',
  'image/x-ms-bmp': 'bmp',
  'image/svg+xml': 'svg',
  'application/java-archive': 'jar',
  'application/msword': 'doc',
  'application/postscript': 'ps',
  'application/vnd.ms-excel': 'xls',
  'application/vnd.ms-powerpoint': 'ppt',
  'application/vnd.apple.mpegurl': 'm3u8',
  'application/dash+xml': 'mpd',
  'application/x-mpegURL': 'm3u8',
  'application/x-7z-compressed': '7z',
  'application/x-rar-compressed': 'rar',
  'application/x-shockwave-flash': 'swf',
  'application/x-xpinstall': 'xpi',
  'application/xhtml+xml': 'xhtml',
  'application/octet-stream': 'bin',
  'application/binary': 'exe',
  'audio/mpeg': 'mp3',
  'audio/mpegurl': 'm3u8',
  'audio/x-mpegurl': 'm3u8',
  'video/3gpp': '3gp',
  'video/mpeg': 'mpg',
  'video/quicktime': 'mov',
  'video/x-flv': 'flv',
  'video/x-mng': 'mng',
  'video/x-ms-asf': 'asf',
  'video/x-ms-wmv': 'wmv',
  'video/x-msvideo': 'avi'
};

function guess(resp, meta = {}) {
  const href = resp.url.split('#')[0].split('?')[0];

  const disposition = resp.headers.get('Content-Disposition');
  let name = '';
  if (disposition) {
    const tmp = /filename\*=UTF-8''([^;]*)/.exec(disposition);
    if (tmp && tmp.length) {
      name = tmp[1].replace(/["']$/, '').replace(/^["']/, '');
      name = decodeURIComponent(name);
    }
  }
  if (!name && disposition) {
    const tmp = /filename=([^;]*)/.exec(disposition);
    if (tmp && tmp.length) {
      name = tmp[1].replace(/["']$/, '').replace(/^["']/, '');
    }
  }
  if (!name) {
    // 处理data URL
    if (href.startsWith('data:')) {
      const mime = href.split('data:')[1].split(';')[0];
      meta.ext = (MIME_TYPES[mime] || mime.split('/')[1] || '').split(';')[0];
      meta.name = 'data_url';
      meta.mime = mime;
      return;
    }
    else {
      const fe = (href.substring(href.lastIndexOf('/') + 1) || 'unknown').slice(-100);
      name = fe;
    }
  }
  name = name || 'unknown';
  // valid file extension "*.webvtt"
  const e = /(.+)\.([^.]{1,6})*$/.exec(name);

  name = e ? e[1] : name;
  meta.mime = resp.headers.get('Content-Type') || meta.mime || '';
  meta.ext = e ? e[2] : (MIME_TYPES[meta.mime] || meta.mime.split('/')[1] || '').split(';')[0];
  meta.ext = meta.ext.slice(0, 15); // cannot be longer than 16 characters.
  //
  meta.name = decodeURIComponent(name) || meta.name;
}

/** Dependency-free ZIP (stored entries, CRC32, UTF-8 filenames). The gallery
 * has already fetched complete media blobs; no compression or external service. */
import { t } from '../shared/i18n.js';
const encoder = new TextEncoder();
const table = new Uint32Array(256);
for (let n = 0; n < 256; n += 1) {
  let c = n;
  for (let bit = 0; bit < 8; bit += 1) c = c & 1 ? (c >>> 1) ^ 0xedb88320 : c >>> 1;
  table[n] = c >>> 0;
}
function crc32(bytes) {
  let crc = -1;
  for (const value of bytes) crc = table[(crc ^ value) & 255] ^ (crc >>> 8);
  return (crc ^ -1) >>> 0;
}
function header(size) { return new Uint8Array(size); }
function u16(view, offset, n) { view.setUint16(offset,n,true); }
function u32(view, offset, n) { view.setUint32(offset,n,true); }
function dosTime(date) {
  const year = Math.max(1980,date.getFullYear());
  return {time:(date.getHours()<<11)|(date.getMinutes()<<5)|(date.getSeconds()>>1),
    date:((year-1980)<<9)|((date.getMonth()+1)<<5)|date.getDate()};
}
export function storedZip(files) {
  if (!files.length || files.length > 65535) throw Error(t('zipItemsLimit'));
  const chunks = [], directory = [];
  let offset = 0;
  const clock = dosTime(new Date());
  for (const file of files) {
    const name = encoder.encode(file.name);
    const data = file.bytes instanceof Uint8Array ? file.bytes : new Uint8Array(file.bytes);
    if (!name.length || name.length > 65535 || data.length > 0xffffffff) throw Error('Invalid ZIP item.');
    const checksum = crc32(data);
    const local = header(30 + name.length), l = new DataView(local.buffer);
    u32(l,0,0x04034b50);u16(l,4,20);u16(l,6,0x0800);u16(l,8,0);
    u16(l,10,clock.time);u16(l,12,clock.date);u32(l,14,checksum);
    u32(l,18,data.length);u32(l,22,data.length);u16(l,26,name.length);
    local.set(name,30);
    chunks.push(local,data);
    const central = header(46 + name.length), c = new DataView(central.buffer);
    u32(c,0,0x02014b50);u16(c,4,20);u16(c,6,20);u16(c,8,0x0800);
    u16(c,10,0);u16(c,12,clock.time);u16(c,14,clock.date);
    u32(c,16,checksum);u32(c,20,data.length);u32(c,24,data.length);
    u16(c,28,name.length);u32(c,42,offset);central.set(name,46);
    directory.push(central);
    offset += local.length + data.length;
  }
  const cdSize = directory.reduce((sum,chunk)=>sum+chunk.length,0);
  if (offset + cdSize + 22 > 0xffffffff) throw Error(t('zipSizeLimit'));
  const end=header(22),e=new DataView(end.buffer);
  u32(e,0,0x06054b50);u16(e,8,files.length);u16(e,10,files.length);
  u32(e,12,cdSize);u32(e,16,offset);
  return new Blob([...chunks,...directory,end],{type:'application/zip'});
}

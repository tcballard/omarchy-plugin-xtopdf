#!/usr/bin/env node
// Protocol peer only: this is NOT a browser and cannot establish PDF fidelity.
const fs = require('node:fs');
const profile = process.argv.find(a => a.startsWith('--user-data-dir=')).slice('--user-data-dir='.length);
const input = fs.createReadStream(null, {fd: 3});
let buffer = Buffer.alloc(0);
input.on('data', chunk => {
  buffer = Buffer.concat([buffer, chunk]);
  let end;
  while ((end = buffer.indexOf(0)) !== -1) {
    const request = JSON.parse(buffer.subarray(0, end));
    buffer = buffer.subarray(end + 1);
    fs.appendFileSync(profile + '/calls.jsonl', JSON.stringify(request) + '\n');
    const result = {
      'Target.createTarget': {targetId:'target-1'},
      'Target.attachToTarget': {sessionId:'session-1'},
      'Page.getFrameTree': {frameTree:{frame:{id:'frame-1'}}},
      'Page.printToPDF': {data:Buffer.from('%PDF-mock').toString('base64')}
    }[request.method] || {};
    // Unsolicited event precedes a deliberately split response.
    fs.writeSync(4, JSON.stringify({method:'Page.frameNavigated',params:{}}) + '\0');
    const response = JSON.stringify(request.method === 'Reject.me'
      ? {id:request.id,error:{message:'Rejected'}} : {id:request.id,result}) + '\0';
    fs.writeSync(4, response.slice(0, 7)); fs.writeSync(4, response.slice(7));
  }
});

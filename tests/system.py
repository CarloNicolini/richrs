#!/usr/bin/env python3
"""Integration tests requiring local sockets and a pseudo-terminal (macOS/Linux)."""
import argparse
import fcntl
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import termios
import threading
import time
import unittest

parser=argparse.ArgumentParser()
parser.add_argument('--binary',type=Path,default=Path('target/debug/rich'))
args,remaining=parser.parse_known_args()
BINARY=str(args.binary.resolve())

class Handler(BaseHTTPRequestHandler):
    def log_message(self,*args):
        pass
    def do_GET(self):
        if self.path=='/redirect':
            self.send_response(302)
            self.send_header('Location','/data.json')
            self.end_headers()
            return
        if self.path=='/missing':
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header('Content-Type','application/json')
        self.end_headers()
        self.wfile.write(b'{"answer":42}')

class SystemTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server=ThreadingHTTPServer(('127.0.0.1',0),Handler)
        cls.thread=threading.Thread(target=cls.server.serve_forever,daemon=True)
        cls.thread.start()
    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
    def test_http_redirect_mime_and_errors(self):
        for path in ['/data.json','/redirect','/no-extension']:
            r=subprocess.run([BINARY,f'http://127.0.0.1:{self.server.server_port}{path}'],capture_output=True,text=True)
            self.assertEqual(r.returncode,0,r.stderr)
            self.assertIn('"answer"',r.stdout)
        r=subprocess.run([BINARY,f'http://127.0.0.1:{self.server.server_port}/missing'],capture_output=True,text=True)
        self.assertNotEqual(r.returncode,0)
        self.assertIn('404',r.stderr)
    def test_broken_pipe_is_success(self):
        r=subprocess.Popen([BINARY,'--print','hello'],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        r.stdout.close()
        self.assertEqual(r.wait(timeout=5),0)
        r.stderr.close()
    def test_native_pager_enters_exits_and_restores_terminal(self):
        master,slave=pty.openpty()
        self.addCleanup(os.close,master)
        self.addCleanup(os.close,slave)
        fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',12,60,0,0))
        before=termios.tcgetattr(slave)
        env=dict(os.environ,TERM='xterm-256color')
        env.pop('PAGER',None)
        env.pop('RICH_THEME',None)
        child=subprocess.Popen([BINARY,'--print','hello\n'*50,'--pager'],stdin=slave,stdout=slave,stderr=slave,env=env)
        def cleanup():
            if child.poll() is None:
                child.kill()
            child.wait()
        self.addCleanup(cleanup)
        buf=b''
        deadline=time.monotonic()+8
        while b'q quit' not in buf and time.monotonic()<deadline:
            ready,_,_=select.select([master],[],[],0.2)
            if ready:
                buf+=os.read(master,65536)
        self.assertIn(b'q quit',buf)
        os.write(master,b'j\x1b[Fq')
        self.assertEqual(child.wait(timeout=8),0)
        self.assertEqual(termios.tcgetattr(slave),before)
        self.assertIn(b'\x1b[?1049h',buf)
    def test_no_color_and_force_terminal(self):
        env=dict(os.environ,NO_COLOR='1',TERM='xterm-256color')
        r=subprocess.run([BINARY,'--print','[red]hello[/]'],capture_output=True,env=env)
        self.assertNotIn(b'\x1b[',r.stdout)
        r=subprocess.run([BINARY,'--print','[red]hello[/]','--force-terminal'],capture_output=True,env=env)
        self.assertIn(b'\x1b[',r.stdout)

if __name__=='__main__':
    unittest.main(argv=[__file__,*remaining])

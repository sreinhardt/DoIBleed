#!env python3

import socket, struct, re, time

#HOST = "localhost"
HOST = "f80ac865e59fcd25817722efd0225048.bsides.40bytectf.com"
PORT = 22380
MSG_SIZE = 100
MAX_LVL = 10
CHAL_RE = re.compile(r"\s*(\-?\d+)\s+([\+\-\*\/\%])\s+(-?\d+)\s+")

def recv_msg(sock):
    msg = sock.recv(MSG_SIZE)
    len = msg[0]
    msg = msg[1:]
    print("recv: {}".format(msg.decode('utf-8')))
    return msg

def send_msg(sock, msg):
    length = struct.pack("<B", len(msg))
    msg = length+bytes(msg, 'utf-8')
    print("send: {}".format(msg))
    sock.send(msg)

def send_msg_over(sock, msg):
    length = struct.pack("<B", 100)
    msg = length+bytes(msg, 'utf-8')
    print("send: {}".format(msg))
    sock.send(msg)

def parse_chal(msg):
    match = CHAL_RE.search(msg.decode('utf-8'))
    left = int(match.group(1))
    oper = match.group(2)
    right = int(match.group(3))
    ops = {
        '+': int(left + right),
        '-': int(left - right),
        '*': int(left * right),
        '/': int(left / right),
        '%': int(left % right),
    }

    if oper not in ops.keys():
        raise RuntimeError("Invalid operation {}".format(oper))
    return ops[oper]

def solve(over=False, delay=0):
    send = send_msg_over if over else send_msg
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.connect((HOST, PORT))
    msg = recv_msg(s)
    send(s, " ")
    count = 1
    while count <= MAX_LVL:
        msg = recv_msg(s)
        val = parse_chal(msg)
        time.sleep(delay)
        send(s, str(val))
        count+=1
    print("Made it {} rounds".format(MAX_LVL))
    recv_msg(s)

if __name__ == "__main__":
    print("\nSolving without flag printed")
    solve()
    print("\n\nSolving with flag printed")
    solve(True)
    print("\n\nFailing to solve w/ timeout")
    solve(delay=3)

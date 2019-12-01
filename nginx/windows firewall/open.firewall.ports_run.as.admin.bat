netsh advfirewall firewall add rule name="TCP Port 4444 in" dir=in action=allow protocol=TCP localport=4444
netsh advfirewall firewall add rule name="TCP Port 4444 out" dir=out action=allow protocol=TCP localport=4444
netsh advfirewall firewall add rule name="TCP Port 1935 in" dir=in action=allow protocol=TCP localport=1935
netsh advfirewall firewall add rule name="TCP Port 1935 out" dir=out action=allow protocol=TCP localport=1935
exit

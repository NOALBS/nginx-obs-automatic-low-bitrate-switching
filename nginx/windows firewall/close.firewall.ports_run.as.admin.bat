netsh advfirewall firewall delete rule name="TCP Port 4444 in" protocol=TCP localport=4444
netsh advfirewall firewall delete rule name="TCP Port 4444 out" protocol=TCP localport=4444
netsh advfirewall firewall delete rule name="TCP Port 1935 in" protocol=TCP localport=1935
netsh advfirewall firewall delete rule name="TCP Port 1935 out" protocol=TCP localport=1935
pause
exit

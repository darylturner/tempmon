# enabling the 1-wire bus

in /boot/config.txt
```
dtoverlay=w1-gpio
```

```
modprobe w1-gpio w1-therm
```


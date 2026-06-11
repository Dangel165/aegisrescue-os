#!/usr/bin/env bash
# Console/login setup for Live ISO — multi-user target, tty1 autologin, no DM required.
configure_live_console() {
    local chroot="$1"

    mkdir -p "${chroot}/etc/systemd/system/getty@tty1.service.d"
    cat > "${chroot}/etc/systemd/system/getty@tty1.service.d/autologin.conf" <<'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin aegis --noclear --keep-baud 115200,38400,9600 %I $TERM
Type=idle
Restart=always
RestartSec=0
EOF

    chroot "${chroot}" useradd -m -s /bin/bash -G sudo aegis 2>/dev/null || true
    echo "aegis:aegis" | chroot "${chroot}" chpasswd 2>/dev/null || true

    mkdir -p "${chroot}/home/aegis"
    cat > "${chroot}/home/aegis/.bash_profile" <<'EOF'
if [ -f /etc/motd-aegis ]; then cat /etc/motd-aegis; fi
echo ""
echo "  AegisRescue Beta — CLI: aegis   GUI: aegiscenter"
echo ""
EOF
    chroot "${chroot}" chown aegis:aegis /home/aegis/.bash_profile 2>/dev/null || true

    cat > "${chroot}/etc/motd-aegis" <<'EOF'

  AegisRescue Beta 0.1.0 — Live Recovery OS
  User: aegis (auto-login on tty1)  |  Alt+F2/F3: other consoles

EOF

    chroot "${chroot}" systemctl set-default multi-user.target 2>/dev/null || true
    chroot "${chroot}" systemctl enable getty@tty1.service 2>/dev/null || true
    chroot "${chroot}" systemctl mask plymouth-quit-wait.service 2>/dev/null || true
}

# Shared kernel cmdline fragment for all Live boot entries.
LIVE_KERNEL_BASE="boot=live components live-media-path=/live nomodeset console=tty1 plymouth.enable=0"

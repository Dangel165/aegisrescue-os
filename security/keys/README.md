# AegisRescue 서명 키 관리

## 구조

```
keys/
├── README.md
├── signing/           # 코드 서명 키 (공개키만 커밋, 비밀키는 별도 HSM 보관)
│   ├── aegis-kernel.pub    # 커널 서명 공개키
│   ├── aegis-module.pub    # 모듈 서명 공개키
│   ├── aegis-pkg.pub       # 패키지 서명 공개키
│   └── aegis-update.pub    # 업데이트 번들 서명 공개키
├── mok/               # Machine Owner Key (Secure Boot MOK 등록용)
│   └── MOK.der             # DER 형식 공개키
└── verify/            # 검증 스크립트
    └── verify-sig.sh
```

## 키 생성 (빌드 서버에서 한 번만)

```bash
# 커널/모듈 서명 키 쌍 생성
openssl req -new -nodes -utf8 -sha512 -days 36500 \
    -subj "/CN=AegisRescue Kernel Signing Key" \
    -x509 -keyout signing/aegis-kernel.pem \
    -out signing/aegis-kernel.crt

# MOK용 DER 변환
openssl x509 -in signing/aegis-kernel.crt -outform DER -out mok/MOK.der

# 공개키만 커밋
cp signing/aegis-kernel.crt signing/aegis-kernel.pub
# 비밀키(.pem)는 절대 커밋 금지
```

## Secure Boot MOK 등록 (사용자 PC에서)

```bash
sudo mokutil --import mok/MOK.der
# 재부팅 후 MOK Manager에서 등록 확인
```

## 커널 서명

```bash
scripts/sign-file sha512 signing/aegis-kernel.pem signing/aegis-kernel.crt vmlinuz
```

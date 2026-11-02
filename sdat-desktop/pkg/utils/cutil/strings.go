package cutil

import (
	"bytes"
	"unsafe"
)

func ParseNullTerminatedString(s string) string {
	ptr := uintptr(unsafe.Pointer(unsafe.StringData(s)))
	buff := new(bytes.Buffer)

	for i := uintptr(0); *(*byte)(unsafe.Pointer(ptr + i)) != 0; i += unsafe.Sizeof(byte(0)) {
		buff.WriteByte(*(*byte)(unsafe.Pointer(ptr + i)))
	}

	return buff.String()
}

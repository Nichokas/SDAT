package g

import "fmt"

func TruncateFloat(f float64, precision int) string {
	format := "%." + fmt.Sprintf("%d", precision) + "f"
	res := fmt.Sprintf(format, f)
	for res[len(res)-1] == '0' {
		res = res[:len(res)-1]
	}

	if res[len(res)-1] == '.' {
		res = res[:len(res)-1]
	}

	return res
}

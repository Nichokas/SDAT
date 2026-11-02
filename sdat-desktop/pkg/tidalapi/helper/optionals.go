package helper

func OptionalString(value string, fallback string) string {
	if value == "" {
		return fallback
	}
	return value
}

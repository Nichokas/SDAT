package g

func Map[T any, U any](slice []T, cb func(T) U) []U {
	result := make([]U, len(slice))
	for i, v := range slice {
		result[i] = cb(v)
	}
	return result
}

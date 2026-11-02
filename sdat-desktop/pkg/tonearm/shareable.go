package tonearm

type Shareable interface {
	// URL returns the shareable URL for the object
	URL() string
}

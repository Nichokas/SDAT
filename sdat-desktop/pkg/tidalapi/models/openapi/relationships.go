package openapi

type Relationship struct {
	ID   string         `json:"id"`
	Meta map[string]any `json:"meta,omitempty"`
	Type string         `json:"type"`
}

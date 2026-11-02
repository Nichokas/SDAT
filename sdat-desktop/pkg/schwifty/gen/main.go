package main

import (
	"fmt"
	"go/parser"
	"go/token"
	"os"
	"regexp"
	"sort"
	"strings"

	_ "embed"
)

//go:embed template/template.go
var template string

//go:embed template/css.go
var css string

//go:embed template/state.go
var state string

var templateList = []string{
	template,
	css,
	state,
}

func main() {
	if len(os.Args) < 3 {
		fmt.Println("Usage: gen <type> <base_type> <package>")
		os.Exit(1)
	}

	typ := os.Args[1]
	baseType := os.Args[2]
	pkg := "schwifty"
	if len(os.Args) > 3 {
		pkg = os.Args[3]
	}

	additionalImports := []string{}
	if strings.Contains(baseType, "adw.") {
		additionalImports = append(additionalImports, "\"codeberg.org/puregotk/puregotk/v4/adw\"")
	}

	parsedTemplates := make([]string, len(templateList))
	for i, template := range templateList {
		parsedTemplates[i] = parseTemplate(template, typ, baseType)
	}

	cleanedTemplates := make([]string, len(parsedTemplates))
	for i, template := range parsedTemplates {
		cleanedTemplates[i] = cleanTemplate(template)
	}

	output := fmt.Sprintf("package %s\n\n", pkg)
	output += "import (\n"
	output += generateFinalImports(parsedTemplates, additionalImports)
	output += "\n)\n\n"
	output += strings.Join(cleanedTemplates, "\n\n")
	os.WriteFile(strings.ToLower(typ)+"_generated.go", []byte(output), 0644)
}

func parseTemplate(template string, typ string, baseType string) string {
	// Get rid of temporary type definitions
	output := strings.ReplaceAll(template, "type TEMPLATE_BASE_TYPE struct{ gtk.Widget }\n\n", "")

	// Insert real type names
	output = strings.ReplaceAll(output, "TEMPLATE_TYPE", typ)
	output = strings.ReplaceAll(output, "TEMPLATE_BASE_TYPE", baseType)
	return output
}

func parseImports(parsedTemplate string) []string {
	fset := token.NewFileSet()

	file, err := parser.ParseFile(
		fset,
		"",             // filename (can be empty)
		parsedTemplate, // source code as string
		parser.ImportsOnly,
	)
	if err != nil {
		fmt.Printf("Error parsing imports: %v\n", err)
		return nil
	}

	var imports []string
	for _, imp := range file.Imports {
		imports = append(imports, imp.Path.Value)
	}

	return imports
}

func generateFinalImports(parsedTemplates []string, additionalImports []string) string {
	importList := additionalImports
	importSet := make(map[string]bool)
	for _, template := range parsedTemplates {
		templateImports := parseImports(template)
		for _, imp := range templateImports {
			if !importSet[imp] {
				importSet[imp] = true
				importList = append(importList, imp)
			}
		}
	}

	// Sort imports according to go fmt rules
	sort.Strings(importList)

	return "\t" + strings.Join(importList, "\n\t")
}

func cleanTemplate(template string) string {
	// Remove package declaration
	output := strings.ReplaceAll(template, "package schwifty\n", "")

	// Remove imports block using regex
	re := regexp.MustCompile(`(?s)import\s*\([^)]*\)\s*`)
	output = re.ReplaceAllString(output, "")

	// Remove single line imports using regex
	singleImportRe := regexp.MustCompile(`(?m)^import\s+"[^"]*"\s*$`)
	output = singleImportRe.ReplaceAllString(output, "")

	return output
}

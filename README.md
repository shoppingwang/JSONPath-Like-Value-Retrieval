# JSONPath-Like-Value-Retrieval
Support the value retrieve from JSON like the expression first(from_json("This is JSON", "$.otel.resourceSpans[*].resource.attributes[?(@.key=='service.name')].value"))

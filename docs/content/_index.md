+++
template = "landing.html"

[extra.hero]
title = "Welcome to Reqs!"
badge = "v0.0.1"
description = "A simple and fast command-line tool to test URLs from a pipeline"
image = "/images/preview.jpg" # Background image
cta_buttons = [
    { text = "Get Started", url = "/get_started/installation", style = "primary" },
    { text = "View on GitHub", url = "https://github.com/hahwul/reqs", style = "secondary" },
]

[extra.features_section]
title = "Essential Features"
description = "Discover reqs's essential features"

[[extra.features]]
title = "Concurrent Request Processing"
desc = "Send HTTP requests to multiple URLs concurrently with customizable concurrency levels for optimal performance."
icon = "fa-solid fa-bolt"

[[extra.features]]
title = "Multiple HTTP Methods"
desc = "Support for various HTTP methods including GET, POST, PUT, DELETE, and more with custom body data."
icon = "fa-solid fa-network-wired"

[[extra.features]]
title = "Flexible Output Formats"
desc = "Output results in plain text, JSON Lines (JSONL), or CSV format. Perfect for pipeline integration and data analysis."
icon = "fa-solid fa-file-export"

[[extra.features]]
title = "Advanced Filtering"
desc = "Filter results by status code, response body content, or regex patterns. Extract only the data you need."
icon = "fa-solid fa-filter"

[[extra.features]]
title = "MCP Server Mode"
desc = "Model Context Protocol (MCP) server mode for seamless integration with AI tools and assistants."
icon = "fa-solid fa-robot"

[[extra.features]]
title = "Customizable Options"
desc = "Configure custom headers, timeouts, retries, proxies, and more for robust network operations."
icon = "fa-solid fa-sliders"

[extra.final_cta_section]
title = "Contributing"
description = "Reqs is an open-source project made with ❤️. If you want to contribute to this project, please submit a pull request with your cool content!"
button = { text = "View on GitHub", url = "https://github.com/hahwul/reqs" }
+++

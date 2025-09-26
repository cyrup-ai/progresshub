# Acknowledgements

## bandwhich

This project incorporates concepts and code from [bandwhich](https://github.com/imsnif/bandwhich), a terminal bandwidth utilization tool.

**Original Author**: Aram Drevekenin  
**License**: MIT License  
**Source**: https://github.com/imsnif/bandwhich

### Adapted Components

The following components have been adapted from bandwhich for use in progresshub:

1. **Network Interface Detection**: The approach of monitoring all active network interfaces rather than attempting to select a single "primary" interface.

2. **Bandwidth Calculation Algorithms**: Methods for calculating real-time bandwidth usage from network interface statistics.

3. **Multi-Interface Aggregation**: The pattern of aggregating bandwidth data across multiple network interfaces to get total system bandwidth usage.

### MIT License (bandwhich)

```
MIT License

Copyright (c) 2019 Aram Drevekenin

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Additional Thanks

We would also like to thank:

- The Rust community for excellent networking and system libraries
- Contributors to the `sysinfo` crate for cross-platform system information access
- The HuggingFace team for their model hub infrastructure
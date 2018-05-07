Script for generating Mandelbrot set.

Usage:
```
Usage: target/release/mandelbrot FILE SIZE UPPERLEFT LOWERRIGHT

# UPPERLEFT and LOWERRIGHT are points in the complex plane.
```

Example:

```
target/release/mandelbrot samples/mandel__0.png 1000x750 -2.0,2.0 2.0,-2.0
```

![Mandelbrot](https://github.com/matDobek/mandelbrot/blob/master/samples/mandel__0.png "Mandelbrot")


```
target/release/mandelbrot mandel.png 1000x750 -1.20,0.35 -1.0,0.20
```

![Mandelbrot](https://github.com/matDobek/mandelbrot/blob/master/samples/mandel__1.png "Mandelbrot")

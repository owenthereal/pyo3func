import pyo3func

def callback(val):
    print(val)
    return val + " called"

print(pyo3func.start_function(callback))

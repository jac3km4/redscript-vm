
native func Assert(cond: Bool)

native func FailEquality(a: String, b: String)
native func FailInequality(a: String, b: String)

func AssertEq(a: Bool, b: Bool) {
  if NotEquals(a, b) {
    FailEquality(ToString(a), ToString(b));
  }
}
func AssertEq(a: Int8, b: Int8) {
  if NotEquals(a, b) {
    FailEquality(ToString(a), ToString(b));
  }
}
func AssertEq(a: Int16, b: Int16) {
  if NotEquals(a, b) {
    FailEquality(ToString(a), ToString(b));
  }
}
func AssertEq(a: Int32, b: Int32) {
  if NotEquals(a, b) {
    FailEquality(ToString(a), ToString(b));
  }
}
func AssertEq(a: Int64, b: Int64) {
  if NotEquals(a, b) {
    FailEquality(ToString(a), ToString(b));
  }
}
func AssertEq(a: Uint8, b: Uint8) {
  if NotEquals(a, b) {
    FailEquality(ToString(a), ToString(b));
  }
}
func AssertEq(a: Uint16, b: Uint16) {
  if NotEquals(a, b) {
    FailEquality(ToString(a), ToString(b));
  }
}
func AssertEq(a: Uint32, b: Uint32) {
  if NotEquals(a, b) {
    FailEquality(ToString(a), ToString(b));
  }
}
func AssertEq(a: Uint64, b: Uint64) {
  if NotEquals(a, b) {
    FailEquality(ToString(a), ToString(b));
  }
}
func AssertEq(a: Float, b: Float) {
  if NotEquals(a, b) {
    FailEquality(ToString(a), ToString(b));
  }
}
func AssertEq(a: Double, b: Double) {
  if NotEquals(a, b) {
    FailEquality(ToString(a), ToString(b));
  }
}
func AssertEq(a: String, b: String) {
  if NotEquals(a, b) {
    FailEquality(ToString(a), ToString(b));
  }
}
func AssertEq(a: CName, b: CName) {
  if NotEquals(a, b) {
    FailEquality(ToString(a), ToString(b));
  }
}
func AssertEq(a: TweakDBID, b: TweakDBID) {
  if NotEquals(a, b) {
    FailEquality(ToString(a), ToString(b));
  }
}
func AssertEq(a: ResRef, b: ResRef) {
  if NotEquals(a, b) {
    FailEquality(ToString(a), ToString(b));
  }
}

func AssertNeq(a: Bool, b: Bool) {
  if Equals(a, b) {
    FailInequality(ToString(a), ToString(b));
  }
}
func AssertNeq(a: Int8, b: Int8) {
  if Equals(a, b) {
    FailInequality(ToString(a), ToString(b));
  }
}
func AssertNeq(a: Int16, b: Int16) {
  if Equals(a, b) {
    FailInequality(ToString(a), ToString(b));
  }
}
func AssertNeq(a: Int32, b: Int32) {
  if Equals(a, b) {
    FailInequality(ToString(a), ToString(b));
  }
}
func AssertNeq(a: Int64, b: Int64) {
  if Equals(a, b) {
    FailInequality(ToString(a), ToString(b));
  }
}
func AssertNeq(a: Uint8, b: Uint8) {
  if Equals(a, b) {
    FailInequality(ToString(a), ToString(b));
  }
}
func AssertNeq(a: Uint16, b: Uint16) {
  if Equals(a, b) {
    FailInequality(ToString(a), ToString(b));
  }
}
func AssertNeq(a: Uint32, b: Uint32) {
  if Equals(a, b) {
    FailInequality(ToString(a), ToString(b));
  }
}
func AssertNeq(a: Uint64, b: Uint64) {
  if Equals(a, b) {
    FailInequality(ToString(a), ToString(b));
  }
}
func AssertNeq(a: Float, b: Float) {
  if Equals(a, b) {
    FailInequality(ToString(a), ToString(b));
  }
}
func AssertNeq(a: Double, b: Double) {
  if Equals(a, b) {
    FailInequality(ToString(a), ToString(b));
  }
}
func AssertNeq(a: String, b: String) {
  if Equals(a, b) {
    FailInequality(ToString(a), ToString(b));
  }
}
func AssertNeq(a: CName, b: CName) {
  if Equals(a, b) {
    FailInequality(ToString(a), ToString(b));
  }
}
func AssertNeq(a: TweakDBID, b: TweakDBID) {
  if Equals(a, b) {
    FailInequality(ToString(a), ToString(b));
  }
}
func AssertNeq(a: ResRef, b: ResRef) {
  if Equals(a, b) {
    FailInequality(ToString(a), ToString(b));
  }
}

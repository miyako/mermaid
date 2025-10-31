//%attributes = {"invisible":true}
#DECLARE($params : Object)

If (Count parameters:C259=0)
	
	//execute in a worker to process callbacks
	CALL WORKER:C1389(1; Current method name:C684; {})
	
Else 
	
	var $mermaid : cs:C1710.mermaid
	$mermaid:=cs:C1710.mermaid.new()
	
	var $tasks : Collection
	$tasks:=[]
	
	var $file : Variant
	
	$file:="graph TD\n    A[Start] --> B{Is it working?}\n    B -- Yes --> C[Great!]\n    B -- No --> D[Check the code]\n    D --> B\n    C --> E[End]"
	$tasks.push({file: $file})
	
	$file:="graph TD\n    A[Start] --> B{Is it working?}\n    B -- Yes --> C[Great!]\n    B -- No --> D[Check the code]\n    D --> B\n    C --> E[End]"
	$tasks.push({file: $file})
	
	$results:=$mermaid.render($tasks)
	
End if 